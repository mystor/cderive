#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
extern crate cexpr;
extern crate regex;
extern crate clang_sys;
extern crate quote;

mod clang;

pub use clang::Cursor;

use std::sync::Arc;
use std::collections::HashMap;

const DERIVE: &'static str = "DERIVE";

fn ensure_libclang_is_loaded() {
    if clang_sys::is_loaded() {
        return;
    }

    // XXX (issue #350): Ensure that our dynamically loaded `libclang`
    // doesn't get dropped prematurely, nor is loaded multiple times
    // across different threads.

    lazy_static! {
        static ref LIBCLANG: Arc<clang_sys::SharedLibrary> = {
            clang_sys::load().expect("Unable to find libclang");
            clang_sys::get_library()
                .expect("We just loaded libclang and it had better still be \
                         here!")
        };
    }

    clang_sys::set_library(Some(LIBCLANG.clone()));
}

/// Try to get the value at the cursor as an attribute(annotate). Returns the
/// name of the value if the cursor points at an attribute(annotate), and
/// Err(()) otherwise.
fn get_annotation(cursor: Cursor) -> Result<String, ()> {
    if cursor.kind() != clang_sys::CXCursor_AnnotateAttr {
        return Err(());
    }
    Ok(cursor.display_name())
}

fn get_derive_name(cursor: Cursor) -> Result<String, ()> {
    let annotation = get_annotation(cursor)?;
    let mut it = annotation.splitn(2, '=');
    let before = it.next().ok_or(())?;
    if before != DERIVE {
        return Err(());
    }
    Ok(it.next().ok_or(())?.to_owned())
}

fn discover_derives<F>(parent_cursor: Cursor, f: &mut F)
    where F: FnMut(Cursor, String)
{
    parent_cursor.visit(|cursor| {
        if let Ok(derive_name) = get_derive_name(cursor) {
            // XXX: Error Handling
            let ty = parent_cursor.typedef_type().unwrap();
            f(*ty.canonical_declaration(None).unwrap().cursor(), derive_name);
        }
        discover_derives(cursor, &mut *f);
        clang_sys::CXChildVisit_Continue
    });
}

pub trait Derive {
    fn derive(&mut self, cursor: Cursor) -> Result<String, ()>;
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

    pub fn run(&mut self, args: &[String]) -> Result<String, ()> {
        ensure_libclang_is_loaded();

        // XXX: Handle this better?
        let filename = &args[0];
        let args = &args[1..];

        let index = clang::Index::new(false, true);
        let file = clang::TranslationUnit::parse(
            &index,
            filename,
            args,
            /* unsaved files */ &[],
            /* opts */ clang_sys::CXTranslationUnit_DetailedPreprocessingRecord,
        ).ok_or(())?;

        let cursor = file.cursor();

        let mut result = String::new();
        discover_derives(cursor, &mut |cursor, name| {
            if let Some(derive) = self.derives.get_mut(&name) {
                let r = derive.derive(cursor)
                    .expect(&format!("Derive {} failed on {:?}", name, cursor));
                result.push_str(&r);
            } else {
                eprintln!("Use of unregistered derive {}", name);
            }
        });

        Ok(result)
    }
}
