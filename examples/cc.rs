extern crate cderive;

use cderive::clang::*;
use std::env;

struct CycleCollect;

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

// Gets the target type of a refptr. Returns None if the type is not a RefPtr or nsCOMPtr
fn refptr_target(ty: Type) -> Option<Type> {
    // If any of these fail, we aren't looking at a RefPtr or nsCOMPtr
    let decl = try_opt!(ty.get_declaration());
    let template = try_opt!(decl.get_template());
    let template_name = try_opt!(template.get_name());
    // XXX: Handle more types?
    if template_name != "RefPtr" && template_name != "nsCOMPtr" {
        return None;
    }
    let target = try_opt!(ty.get_template_argument_types());
    if target.is_empty() {
        return None;
    }
    let target = try_opt!(target[0]);
    Some(target)
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

impl cderive::Derive for CycleCollect {
    fn derive(&mut self, entity: Entity) -> Result<String, ()> {
        let fields = fields(entity);
        eprintln!("{:?}", fields);

        for field in fields {
            let ty = try_opt!(field.get_type(), continue);
            let target = try_opt!(refptr_target(ty), continue);

            if let Some(decl) = target.get_declaration() {
                
            }

        }

        let typename = entity.get_display_name().unwrap();
        let res = format!("
{ty}::cycleCollection::Unlink(void* p) {{
  {ty} *tmp = DowncastCCParticipant<{ty}>(p);
  {body}
}}", ty = typename, body = "");

        eprintln!("{}", res);

        Ok("".to_owned())
        /*
        Ok(format!("
int {ty}::field_count() {{
  return {count};
}}

const char** {ty}::field_names() {{
  static const char* FIELD_NAMES[] = {{ {names} }};
  return FIELD_NAMES;
}}
",
            ty = entity.get_display_name().unwrap(),
            count = fields.len(),
            names = fields.join(", ")
        ))
        */
    }
}

pub fn main() {
    let mut cyclecollect = CycleCollect;

    let mut deriver = cderive::Deriver::new();
    deriver.register("CycleCollection", &mut cyclecollect);

    let result = deriver.run(&env::args().skip(1).collect::<Vec<_>>()).unwrap();
    println!("{}", result);
}
