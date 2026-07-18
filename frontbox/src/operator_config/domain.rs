use std::fmt::Debug;

pub trait Domain<T> {
  fn inc(&self, value: &T) -> T;
  fn dec(&self, value: &T) -> T;
}

impl<T> Debug for dyn Domain<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Domain")
  }
}

#[derive(Clone, Debug)]
pub struct Choices<T> {
  values: Vec<T>,
}

impl<T> Choices<T>
where
  T: PartialEq,
{
  fn find(&self, value: &T) -> usize {
    self
      .values
      .iter()
      .position(|x| x == value)
      .unwrap_or(self.values.len() - 1)
  }
}

impl<T> Domain<T> for Choices<T>
where
  T: PartialEq + Clone,
{
  fn inc(&self, value: &T) -> T {
    let mut index = self.find(value).saturating_add(1);

    if index >= self.values.len() {
      index = self.values.len() - 1;
    }

    self.values[index].clone()
  }

  fn dec(&self, value: &T) -> T {
    let index = self.find(value).saturating_sub(1);
    self.values[index].clone()
  }
}
