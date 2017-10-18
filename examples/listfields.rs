extern crate cderive;

use cderive::clang::*;
use std::env;

struct ListFields;

impl cderive::Derive for ListFields {
    fn derive(&mut self, entity: Entity) -> Result<String, ()> {
        let mut fields = Vec::new();
        entity.visit_children(|entity, _| {
            if entity.get_kind() == EntityKind::FieldDecl {
                fields.push(format!("{:?}", entity.get_display_name().unwrap()));
            }
            EntityVisitResult::Continue
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
            ty = entity.get_display_name().unwrap(),
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
