use async_trait::async_trait;
use colored::*;
use tokio::time::sleep;
use yaml_rust::Yaml;

use crate::actions::extract;
use crate::actions::Runnable;
use crate::benchmark::{Context, Pool, Reports};
use crate::config::Config;

use std::convert::TryFrom;
use std::time::Duration;

#[derive(Clone)]
pub struct Delay {
  name: String,
  seconds: f64,
}

impl Delay {
  pub fn is_that_you(item: &Yaml) -> bool {
    item["delay"].as_hash().is_some()
  }

  pub fn new(item: &Yaml, _with_item: Option<Yaml>) -> Delay {
    let name = extract(item, "name");
    let seconds = f64::try_from(item["delay"]["seconds"].as_f64().unwrap()).expect("Invalid number of seconds");

    Delay {
      name,
      seconds,
    }
  }
}

#[async_trait]
impl Runnable for Delay {
  async fn execute(&self, _context: &mut Context, _reports: &mut Reports, _pool: &Pool, config: &Config) {
    sleep(Duration::from_secs_f64(self.seconds)).await;

    if !config.quiet {
      println!("{:width$} {}{}", self.name.green(), self.seconds.to_string().cyan().bold(), "s".magenta(), width = 25);
    }
  }
}

#[cfg(test)]
mod tests {
  use yaml_rust::{Yaml, YamlLoader};
  use std::time::Instant;
  use tokio_test;

use super::*;
  #[test]
  fn test_delay() {
    let delay_seconds= 0.1;
    let start = Instant::now();
    let mock_bench = format!("{{\"name\": \"test\", \"delay\": {{ \"seconds\": {} }} }}",delay_seconds);
    //.to_string());
    let item = YamlLoader::load_from_str(mock_bench.as_str()).unwrap();
    let with_item = Some(Yaml::String("{}".to_string()));
    let delay = Delay::new(&item[0], with_item);
    let context: &mut Context = { "a": "abc"};
    let reports = Reports;
    let pool = Pool;
    let config = Config;
    tokio_test::block_on(delay.execute(context, reports, pool, config));
    let elapsed = start.elapsed().as_secs_f64();
    println!("test {} > {}", elapsed, delay_seconds);
    assert!( elapsed > delay_seconds);
  }
}
