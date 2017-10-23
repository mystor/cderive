extern crate cderive;

use std::env;

mod cc;

// XXX: This is not a good entry point yet
pub fn main() {
    // XXX: This is super gross
    let mut cyclecollect = cc::CycleCollect;

    let mut deriver = cderive::Deriver::new();
    deriver.register("CycleCollection", &mut cyclecollect);

    let args = env::args().skip(1)
        .filter(|a| !a.starts_with("-W"))
        .collect::<Vec<_>>();

    let result = deriver.run(&args).unwrap();
    println!("{}", result);
}
