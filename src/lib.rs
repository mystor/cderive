extern crate clang_sys; // NOTE: This should match the version used by clang ideally
pub extern crate clang;

use clang::*;

use std::collections::HashMap;
use std::error::Error;

const DERIVE: &'static str = "DERIVE";

/// Try to get the value at the cursor as an attribute(annotate). Returns the
/// name of the value if the cursor points at an attribute(annotate), and
/// Err(()) otherwise.
fn get_annotation(entity: Entity) -> Result<String, ()> {
    if entity.get_kind() != EntityKind::AnnotateAttr {
        return Err(());
    }
    entity.get_display_name().ok_or(())
}

fn get_derive_name(entity: Entity) -> Result<String, ()> {
    let annotation = get_annotation(entity)?;
    let mut it = annotation.splitn(2, '=');
    let before = it.next().ok_or(())?;
    if before != DERIVE {
        return Err(());
    }
    Ok(it.next().ok_or(())?.to_owned())
}

fn discover_derives<F>(outer: Entity, f: &mut F)
    where F: FnMut(Entity, String)
{
    outer.visit_children(|entity, parent_entity| {
        if let Ok(derive_name) = get_derive_name(entity) {
            // XXX: Error Handling
            let ty = parent_entity.get_typedef_underlying_type().unwrap();
            f(ty.get_declaration().unwrap().get_definition().unwrap(), derive_name);
        }
        EntityVisitResult::Recurse
    });
}

// This is copied mostly wholesale from rust-bindgen
fn fixup_clang_args(args: &mut Vec<String>) {
    // Filter out include paths and similar stuff, so we don't incorrectly
    // promote them to `-isystem`.
    let sysargs = {
        let mut last_was_include_prefix = false;
        args.iter().filter(|arg| {
            if last_was_include_prefix {
                last_was_include_prefix = false;
                return false;
            }

            let arg = &**arg;

            // https://clang.llvm.org/docs/ClangCommandLineReference.html
            // -isystem and -isystem-after are harmless.
            if arg == "-I" || arg == "--include-directory" {
                last_was_include_prefix = true;
                return false;
            }

            if arg.starts_with("-I") || arg.starts_with("--include-directory=") {
                return false;
            }

            true
        }).cloned().collect::<Vec<_>>()
    };

    if let Some(clang) = clang_sys::support::Clang::find(None, &sysargs) {
        // If --target is specified, assume caller knows what they're doing
        // and don't mess with include paths for them
        let has_target_arg = args
            .iter()
            .rposition(|arg| arg.starts_with("--target"))
            .is_some();
        if !has_target_arg {
            // TODO: distinguish C and C++ paths? C++'s should be enough, I
            // guess.
            if let Some(cpp_search_paths) = clang.cpp_search_paths {
                for path in cpp_search_paths.into_iter() {
                    if let Ok(path) = path.into_os_string().into_string() {
                        args.push("-isystem".to_owned());
                        args.push(path);
                    }
                }
            }
        }
    }
}

pub trait Derive {
    fn derive(&mut self, entity: Entity) -> Result<String, ()>;
}

pub struct Deriver<'a> {
    derives: HashMap<String, &'a mut Derive>,
}

impl<'a> Deriver<'a> {
    pub fn new() -> Self {
        Deriver { derives: HashMap::new() }
    }

    pub fn register<S: Into<String>>(&mut self, name: S, derive: &'a mut Derive) {
        self.derives.insert(name.into(), derive);
    }

    pub fn run(&mut self, args: &[String]) -> Result<String, Box<Error>> {
        let clang = Clang::new()?;
        let index = Index::new(&clang, false, true);

        let filename = &args[0];
        let mut args = args[1..].to_owned();
        fixup_clang_args(&mut args);
        let file = index.parser(filename).arguments(&args).parse()?;

        let entity = file.get_entity();

        // XXX: Encode filename as C-style string?
        let mut result = format!("#include {:?}\n\n", filename);
        discover_derives(entity, &mut |entity, name| {
            if let Some(derive) = self.derives.get_mut(&name) {
                let r = derive.derive(entity)
                    .expect(&format!("Derive {} failed on {:?}", name, entity));
                result.push_str(&r);
            } else {
                eprintln!("Use of unregistered derive {}", name);
            }
        });

        Ok(result)
    }
}
