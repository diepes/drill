use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};

use serde_json::{json, Map, Value};
use tokio;

use crate::actions::{Report, Runnable};
use crate::config::Config;
use crate::expandable::include;
use crate::tags::Tags;
use crate::writer;

use reqwest::Client;

use colored::*;

pub type Benchmark = Vec<Box<(dyn Runnable + Sync + Send)>>;
pub type Context = Map<String, Value>;
pub type Reports = Vec<Report>;
pub type PoolStore = HashMap<String, Client>;
pub type Pool = Arc<Mutex<PoolStore>>;

pub struct BenchmarkResult {
  pub reports: Vec<Reports>,
  pub duration: f64,
}

async fn run_iteration(benchmark: Arc<Benchmark>, pool: Pool, config: Arc<Config>, iteration: i64, log_t: Instant) -> Vec<Report> {
  const FN_LOG: &str = "benchmark::run_iteration";
  log::info!("{} Starting iteration #{} {:.6}s", FN_LOG,iteration,log_t.elapsed().as_secs_f64());
  if config.rampup > 0 {
    let delay = config.rampup / config.iterations;
    log::info!("{} iteration {} sleep for {}", FN_LOG,iteration, (delay * iteration));
    tokio::time::sleep(Duration::new((delay * iteration) as u64, 0)).await;
  }

  let mut context: Context = Context::new();
  let mut reports: Vec<Report> = Vec::new();

  context.insert("iteration".to_string(), json!(iteration.to_string()));
  context.insert("base".to_string(), json!(config.base.to_string()));

  for (i,item) in benchmark.iter().enumerate() {
    log::debug!("{} iteration {} for benchmark.iter execute i={} {:.6}s", FN_LOG,iteration, i, log_t.elapsed().as_secs_f64());
    // fake  await to
    ////tokio::time::sleep(Duration::new( 0, 0)).await;
    item.execute(&mut context, &mut reports, &pool, &config).await;
  }

  reports
}

fn join<S: ToString>(l: Vec<S>, sep: &str) -> String {
  l.iter().fold(
    "".to_string(),
    |a,b| if !a.is_empty() {a+sep} else {a} + &b.to_string()
  )
}

#[allow(clippy::too_many_arguments)]
pub fn execute(benchmark_path: &str, report_path_option: Option<&str>, relaxed_interpolations: bool, no_check_certificate: bool, quiet: bool, nanosec: bool, timeout: u64, verbose: bool, tags: &Tags) -> BenchmarkResult {
  const FN_LOG: &str = "benchmark::execute";
  let log_t = Instant::now();
  log::info!("{} Start {:.3}s",FN_LOG, log_t.elapsed().as_secs_f64());
  let config = Arc::new(Config::new(benchmark_path, relaxed_interpolations, no_check_certificate, quiet, nanosec, timeout, verbose));

  if report_path_option.is_some() {
    println!("{}: {}. Ignoring {} and {} properties...", "Report mode".yellow(), "on".purple(), "concurrency".yellow(), "iterations".yellow());
  } else {
    println!("{} {}", "Concurrency".yellow(), config.concurrency.to_string().purple());
    println!("{} {}", "Iterations".yellow(), config.iterations.to_string().purple());
    println!("{} {}", "Rampup".yellow(), config.rampup.to_string().purple());
  }

  println!("{} {}", "Base URL".yellow(), config.base.purple());
  println!();

  let threads = std::cmp::min(num_cpus::get(), config.concurrency as usize);
  let rt = tokio::runtime::Builder::new_multi_thread().enable_all().worker_threads(threads).build().unwrap();
  log::info!("{} tokio->rt {:.6}s",FN_LOG, log_t.elapsed().as_secs_f64());
  rt.block_on(async {
    let mut benchmark: Benchmark = Benchmark::new();
    let pool_store: PoolStore = PoolStore::new();

    include::expand_from_filepath(benchmark_path, &mut benchmark, Some("plan"), tags);
    log::info!("{} fin expand_from_filepath {:.6}s",FN_LOG, log_t.elapsed().as_secs_f64());
    // PES // log::info!("benchmark {:?}",benchmark);

    if benchmark.is_empty() {
      eprintln!("Empty benchmark. Exiting.");
      std::process::exit(1);
    }

    let benchmark = Arc::new(benchmark);
    let pool = Arc::new(Mutex::new(pool_store));

    if let Some(report_path) = report_path_option {
      let reports = run_iteration(benchmark.clone(), pool.clone(), config, 0, log_t.clone()).await;

      writer::write_file(report_path, join(reports, ""));

      BenchmarkResult {
        reports: vec![],
        duration: 0.0,
      }
    } else {
      log::info!("{} children = map run_iteration {}", FN_LOG,config.iterations);
      let children = (0..config.iterations).map(|iteration| run_iteration(benchmark.clone(), pool.clone(), config.clone(), iteration, log_t.clone()));
      log::info!("{} children = map run_iteration {} done in {:.6}", FN_LOG,config.iterations, log_t.elapsed().as_secs_f64());
      //real quick up to here <8mSec
      let buffered = stream::iter(children).buffer_unordered(config.concurrency as usize);

      let begin = Instant::now();
      let reports: Vec<Vec<Report>> = buffered.collect::<Vec<_>>().await;
      let duration = begin.elapsed().as_secs_f64();

      BenchmarkResult {
        reports,
        duration,
      }
    }
  })
}
