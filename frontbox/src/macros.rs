#[macro_export]
macro_rules! signals {
  ($($signal:expr),* $(,)?) => {
    vec![$(Box::new($signal) as Box<dyn Signal>),*]
  }
}
