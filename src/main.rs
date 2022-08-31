use anyhow::Result;
use env_logger;
use log::{error, warn, info, debug};
use std::{env, fmt::format};
use structopt::{StructOpt, clap::{self, arg_enum}};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use chrono::{DateTime, Local};
use tokio::sync::mpsc::{self, Sender};

#[derive(Debug, Clone, StructOpt)]
#[structopt(name="lambda-test-format")]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
struct Opt {
    #[structopt(short = "0", long = "out_path")]
    output_csv: PathBuf,
    #[structopt(short = "m", long = "mode", possible_values(&Mode::variants()))]
    mode: Mode,
    #[structopt(short = "t", long = "thread")]
    threads: u64,
    #[structopt(short = "n", long = "tasks")]
    tasks: u64,
}

arg_enum! {
    #[derive(Debug, Clone)]
    pub enum Mode {
        Series,
        Parallel,
    }
}

fn write_csv(path: PathBuf, collections: Vec<String>) -> Result<()> {
    let now: DateTime<Local> = Local::now();
    let f = now.format("%Y%m%d%M%H.csv");
    let f = format!("{}/{}", path.display(), f);

    let mut wtr = csv::WriterBuilder::new().from_path(f).unwrap();
    
    for record in collections.into_iter() {
        wtr.serialize(record)?;
    }

    Ok(())
}

async fn run_function(num: String, tx: Sender<String>) -> Result<()> {

    tx.send(format!("thread number: {}", num)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let Opt {
        output_csv,
        mode,
        threads,
        tasks
    } = Opt::from_args();

    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let output_collections: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

    let (tx, mut rx) = mpsc::channel(1000);

    let logging_task = tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            info!("log: {:?}", log);
        }
    });

    match mode {
        Mode::Series => {
            for _ in 1..=tasks {
                if let Err(e) = run_function("1".to_string(), tx.clone()).await {
                    error!("function error: {:?}", e);
                }
            }
        }
        Mode::Parallel => {
            let futures = futures::future::join_all(
                (1..=threads)
                    .map(|i| {
                        let clone_tx = tx.clone();
                        let thread_num = i;

                        tokio::spawn(async move {
                            for _ in 1..=tasks {
                                if let Err(e) = run_function(thread_num.to_string(), clone_tx.clone()).await {
                                    error!("function error: {:?}", e);
                                }
                            }
                        })
                    })

            ).await;

            futures
                .into_iter()
                .for_each(|i| {
                    if let Err(e) = i{
                        error!("join error: {:?}", e);
                    }
                })
        }
    }

    let (logging_task,) = tokio::join!(logging_task);
    logging_task?;

    let lock = output_collections.lock().unwrap();
    write_csv(output_csv, lock.to_vec())?;

    println!("finished lambda test");
    Ok(())
}
