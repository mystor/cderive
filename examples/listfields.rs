extern crate clang_sys;
extern crate cderive;

use cderive::*;
use std::env;

struct ListFields;

impl cderive::Derive for ListFields {
    fn derive(&mut self, cursor: Cursor) -> Result<String, ()> {
        let mut fields = Vec::new();
        cursor.visit(|cursor| {
            if cursor.kind() == clang_sys::CXCursor_FieldDecl {
                fields.push(format!("{:?}", cursor.spelling()));
            }
            clang_sys::CXChildVisit_Continue
        });

        Ok(format!("
int {ty}::field_count() {{
  return {count};
}}

const char** {ty}::field_names() {{
  static const char* FIELD_NAMES[] = {{ {names} }};
  return FIELD_NAMES;
}}
",
            ty = cursor.display_name(),
            count = fields.len(),
            names = fields.join(", ")
        ))
    }
}

pub fn main() {
    let mut listfields = ListFields;

    let mut deriver = cderive::Deriver::new();
    deriver.register("ListFields", &mut listfields);

    let result = deriver.run(&env::args().skip(1).collect::<Vec<_>>()).unwrap();
    println!("{}", result);
}
