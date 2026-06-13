pub mod helper;

pub struct User {
  pub name: String,
}

impl User {
  pub fn new(name: String) -> Self {
    Self { name }
  }
}

pub fn run() {
  let _u = User::new("test".to_string());
  helper::utils();
}
