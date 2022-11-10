use std::collections::HashMap;
use std::path::PathBuf;
use std::{
    fs::{read_dir, File},
    io::BufReader,
};
use std::str::FromStr;

use pageserver_api::models::PagestreamFeMessage;
use utils::id::{TenantId, TimelineId, ConnectionId};

use clap::{Parser, Subcommand};


/// Utils for working with pageserver read traces
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of trace directory
    #[arg(short, long)]
    path: PathBuf,

    #[command(subcommand)]
    command: Command,
}

/// What to do with the read trace
#[derive(Subcommand, Debug)]
enum Command {
    /// List traces in the directory
    List,

    /// Print the traces in text format
    Dump,

    /// Print stats and anomalies about the traces
    Analyze,

    /// Send the read requests to a pageserver
    Replay,
}

// HACK This function will change and improve as we see what kind of analysis is useful.
//      Currently it collects the difference in blkno of consecutive GetPage requests,
//      and counts the frequency of each value. This information is useful in order to:
//      - see how sequential a workload is by seeing how often the delta is 1
//      - detect any prefetching anomalies by looking for negative deltas during seqscan
fn analyze_trace<R: std::io::Read>(mut reader: R) {
    let mut total = 0;
    let mut deltas = HashMap::<i32, u32>::new();
    let mut prev = 0;

    // Compute stats
    while let Ok(msg) = PagestreamFeMessage::parse(&mut reader) {
        match msg {
            PagestreamFeMessage::Exists(_) => {}
            PagestreamFeMessage::Nblocks(_) => {}
            PagestreamFeMessage::GetPage(req) => {
                total += 1;

                let delta = (req.blkno as i32) - (prev as i32);
                prev = req.blkno;

                match deltas.get_mut(&delta) {
                    Some(c) => {*c += 1;},
                    None => {deltas.insert(delta, 1);},
                };
            },
            PagestreamFeMessage::DbSize(_) => {}
        };
    }

    // Print stats.
    let mut other = deltas.len();
    deltas.retain(|_, count| *count > 300);
    other -= deltas.len();
    dbg!(total);
    dbg!(other);
    dbg!(deltas);
}

fn dump_trace<R: std::io::Read>(mut reader: R) {
    while let Ok(msg) = PagestreamFeMessage::parse(&mut reader) {
        println!("{msg:?}");
    }
}

#[derive(Debug)]
struct TraceFile {
    #[allow(dead_code)]
    pub tenant_id: TenantId,

    #[allow(dead_code)]
    pub timeline_id: TimelineId,

    #[allow(dead_code)]
    pub connection_id: ConnectionId,

    pub path: PathBuf,
}

fn get_trace_files(traces_dir: &PathBuf) -> anyhow::Result<Vec<TraceFile>> {
    let mut trace_files = Vec::<TraceFile>::new();

    for tenant_dir in read_dir(traces_dir)? {
        let entry = tenant_dir?;
        let path = entry.path();
        let tenant_id = TenantId::from_str(path.file_name().unwrap().to_str().unwrap())?;

        for timeline_dir in read_dir(path)? {
            let entry = timeline_dir?;
            let path = entry.path();
            let timeline_id = TimelineId::from_str(path.file_name().unwrap().to_str().unwrap())?;

            for trace_dir in read_dir(path)? {
                let entry = trace_dir?;
                let path = entry.path();
                let connection_id = ConnectionId::from_str(path.file_name().unwrap().to_str().unwrap())?;

                trace_files.push(TraceFile {
                    tenant_id,
                    timeline_id,
                    connection_id,
                    path,
                });
            }
        }
    }

    Ok(trace_files)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::List => {
            for trace_file in get_trace_files(&args.path)? {
                println!("{trace_file:?}");
            }
        }
        Command::Dump => {
            for trace_file in get_trace_files(&args.path)? {
                let file = File::open(trace_file.path.clone())?;
                let reader = BufReader::new(file);
                dump_trace(reader);
            }
        }
        Command::Analyze => {
            for trace_file in get_trace_files(&args.path)? {
                println!("analyzing {trace_file:?}");
                let file = File::open(trace_file.path.clone())?;
                let reader = BufReader::new(file);
                analyze_trace(reader);
            }
        },
        Command::Replay => todo!(),
    }

    Ok(())
}
