use cderive::Derive;
use cderive::clang::*;

use std::fmt::Write;

pub struct CycleCollect;

macro_rules! try_opt {
    ($e:expr) => {
        try_opt!($e, return None)
    };
    ($e:expr, $other:expr) => {
        match $e {
            Some(x) => x,
            None => $other,
        }
    };
}

// These flags control how we determine which fields need to be cycle collected.
const CC_REFCNT_TY: &'static str = "nsCycleCollectingAutoRefCnt";
const CC_CLASSNAME: &'static str = "cycleCollection";
// These are single-argument template types which, when seen, are considered to
// need to be cycle collected if their backing type is a potential CC target.
const REFPTR_TMPLS: [&'static str; 2] = [
    "RefPtr",
    "nsCOMPtr",
    // XXX: nsCOMArray etc?
];
// These are single-argument template types which act as containers for their
// first template argument, and will be traversed if their first argument would
// be traversed.
const CONTAINER_TMPLS: [&'static str; 1] = [
    "nsTArray",
];

// XXX: Handle objects with more than one template parameter?
// XXX: Walk up inheritence chains?
fn single_template_target<F>(ty: Type, mut matches: F) -> Option<Type>
    where F: FnMut(&str) -> bool
{
    // If any of these fail, we aren't looking at a template target
    let decl = try_opt!(ty.get_declaration());
    let template = try_opt!(decl.get_template());
    let template_name = try_opt!(template.get_name());
    if !matches(&template_name) {
        return None;
    }
    let target = try_opt!(ty.get_template_argument_types());
    if target.is_empty() {
        return None;
    }
    let target = try_opt!(target[0]);
    Some(target)
}

// None if not a refptr, otherwise get the target
fn refptr_target(ty: Type) -> Option<Type> {
    single_template_target(ty, |s| REFPTR_TMPLS.iter().any(|&t| t == s))
}

// None if not a container, otherwise get the target
// XXX: Multi-template-argument containers (e.g. e.g. hashmap)
fn container_target(ty: Type) -> Option<Type> {
    single_template_target(ty, |s| CONTAINER_TMPLS.iter().any(|&t| t == s))
}

fn fields(entity: Entity) -> Vec<Entity> {
    let mut fields = Vec::new();
    entity.visit_children(|entity, _| {
        if entity.get_kind() == EntityKind::FieldDecl {
            fields.push(entity);
        }
        EntityVisitResult::Continue
    });
    fields
}

fn defn_for_ty(ty: Type) -> Option<Entity> {
    ty.get_declaration().and_then(|d| d.get_definition())
}

// Run pred on each base. If any of the preds return Some(v), return that value,
// otherwise return None.
fn base_matching<'a, F, T>(entity: Entity<'a>, pred: &mut F) -> Option<T>
    where F: FnMut(Entity<'a>) -> Option<T>
{
    if let Some(r) = pred(entity) {
        return Some(r);
    }

    let mut result = None;
    entity.visit_children(|child, _| {
        if child.get_kind() == EntityKind::BaseSpecifier {
            if let Some(base) = defn_for_ty(child.get_type().unwrap()) {
                if let Some(r) = pred(base) {
                    result = Some(r);
                    return EntityVisitResult::Break;
                }

                result = base_matching(base, pred);
                if result.is_some() {
                    return EntityVisitResult::Break;
                }
            }
        }
        EntityVisitResult::Continue
    });

    result
}

// Does Entity implement isupports?
fn is_isupports(entity: Entity) -> bool {
    base_matching(entity, &mut |entity| match entity.get_display_name() {
        Some(ref s) if s == "nsISupports" => Some(()),
        _ => None,
    }).is_some()
}

// Get the CC base of Entity. A base is a CC base if it has a CC_CLASSNAME inner
// class.
fn cc_base(orig: Entity) -> Option<Entity> {
    base_matching(orig, &mut |entity| {
        if orig == entity { return None; }

        let mut result = None;
        entity.visit_children(|child, _| {
            if child.get_kind() == EntityKind::ClassDecl {
                match child.get_display_name() {
                    Some(ref s) if s == CC_CLASSNAME => {
                        result = Some(entity);
                        return EntityVisitResult::Break;
                    }
                    _ => {}
                }
            }
            EntityVisitResult::Continue
        });
        result
    })
}

// Get the mRefCnt field of the given type, if it is present. Walk through base
// classes to find it.
fn refcnt_field(entity: Entity) -> Option<Entity> {
    base_matching(entity, &mut |entity| {
        for field in fields(entity) {
            match field.get_display_name() {
                Some(ref s) if s == "mRefCnt" => return Some(field),
                _ => {},
            }
        }
        None
    })
}

// Check if this type should be CCed if behind a RefPtr or similar.
fn cc_ptr_target(ty: Type) -> bool {
    let decl = try_opt!(ty.get_declaration(), return false);
    let decl = try_opt!(decl.get_definition(), return false);

    let rc_field = refcnt_field(decl);
    let rc_field_type = rc_field
        .and_then(|f| f.get_type())
        .map(|t| t.get_display_name());

    // If we see a cycle collecting refcnt, we know we're done!
    match rc_field_type {
        Some(ref ty) if ty == CC_REFCNT_TY => return true,
        _ => {}
    }

    // If we're an nsISupports-base field, and have a rc field, it isn't
    // CC_REFCNT_TY, so the target is not cycle collected.
    is_isupports(decl) && !rc_field_type.is_some()
}

// Check if this type should be CCed
fn should_traverse_unlink(ty: Type) -> bool {
    // Check if we're looking at a RefPtr<T> where T is a cc ptr target
    if let Some(rt) = refptr_target(ty) {
        return cc_ptr_target(rt);
    }

    // We should traverse/unlink a container if its elements can be
    // traversed/unlinked.
    if let Some(rt) = container_target(ty) {
        return should_traverse_unlink(rt);
    }

    false
}

impl Derive for CycleCollect {
    fn derive(&mut self, entity: Entity) -> Result<String, ()> {
        let typename = entity.get_display_name().unwrap();
        let cc_basename = cc_base(entity).and_then(|b| b.get_display_name());

        let mut unlink = String::new();
        let mut traverse = String::new();
        // XXX: trace?

        // Begin blocks
        write!(unlink, "NS_IMPL_CYCLE_COLLECTION_UNLINK_BEGIN({})\n",
               typename);
        if let Some(ref basename) = cc_basename {
            write!(traverse,
                   "NS_IMPL_CYCLE_COLLECTION_TRAVERSE_BEGIN_INHERITED({}, {})\n",
                   typename, basename);
        } else {
            write!(traverse, "NS_IMPL_CYCLE_COLLECTION_TRAVERSE_BEGIN({})\n",
                   typename);
        }

        let fields = fields(entity);
        for field in fields {
            let name = try_opt!(field.get_display_name(), continue);
            let ty = try_opt!(field.get_type(), continue);
            if should_traverse_unlink(ty) {
                use std::fmt::Write;
                write!(unlink, "  NS_IMPL_CYCLE_COLLECTION_UNLINK({})\n", name)
                    .unwrap();
                write!(traverse, "  NS_IMPL_CYCLE_COLLECTION_TRAVERSE({})\n", name)
                    .unwrap();
            }
        }

        // End blocks
        write!(traverse, "NS_IMPL_CYCLE_COLLECTION_TRAVERSE_END\n");
        if let Some(ref basename) = cc_basename {
            write!(unlink, "NS_IMPL_CYCLE_COLLECTION_UNLINK_END_INHERITED({})\n",
                   basename);
        } else {
            write!(unlink, "NS_IMPL_CYCLE_COLLECTION_UNLINK_END\n");
        }

        let typename = entity.get_display_name().unwrap();
        let res = format!("{unlink}\n{traverse}",
                          unlink = unlink,
                          traverse = traverse);

        Ok(res)
    }
}
