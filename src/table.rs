use std::cmp;
use ::std::fmt;
use std::rc::Rc;
use std::iter::FromIterator;

#[derive(Clone)]
struct Field<T> {
  get_field: Rc<fn(&T) -> String>,
  width: usize,
  name: String,
}

impl<T> Field<T> {
  fn sample_width(&mut self, t: &T) {
    self.width = cmp::max((self.get_field)(t).len(), self.width)
  }

  fn format_name(&self) -> String {
    format!("{1:0$}", self.width, self.name)
  }

  fn format(&self, t: &T) -> String {
    format!("{1:0$}", self.width, (self.get_field)(t))
  }
}

#[derive(Clone)]
pub struct Printer<T> {
  fields: Vec<Field<T>>,
  each_row: Rc<fn(&T)>,
}

pub fn printer<T>() -> Printer<T> {
  Printer {
    fields: Vec::new(),
    each_row: Rc::new(|_| ()),
  }
}

impl<T> Printer<T>
  where T: Clone
{
  pub fn field(mut self, name: &str, get_field: fn(&T) -> String) -> Printer<T> {
    self.fields.push(Field {
      name: name.to_owned(),
      get_field: Rc::new(get_field),
      width: name.len(),
    });
    self
  }

  pub fn each_row<F>(&self, f: fn(&T)) -> Self {
    Printer {
      fields: self.fields.clone(),
      each_row: Rc::new(f),
    }
  }

  pub fn with_items<U>(&mut self, ts: U) -> Table<T>
    where T: Clone,
          U: IntoIterator<Item = T> + Clone
  {
    let mut np = self.clone();
    for t in ts.clone() {
      for f in np.fields.iter_mut() {
        f.sample_width(&t)
      }
    }

    return Table {
      printer: np,
      items: Vec::from_iter(ts),
    };
  }
}

pub struct Table<T> {
  printer: Printer<T>,
  items: Vec<T>,
}

impl<T> fmt::Display for Table<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.items.len() == 0 {
      return Ok(());
    }
    let line: String =
      self.printer.fields.iter().map(|f| f.format_name()).collect::<Vec<_>>().join(" | ");
    write!(f, "{}\n", line)?;

    for t in self.items.iter() {
      let line: String =
        self.printer.fields.iter().map(|f| f.format(&t)).collect::<Vec<_>>().join(" | ");
      write!(f, "{}\n", line)?;
      (self.printer.each_row)(t);
    }
    Ok(())
  }
}
