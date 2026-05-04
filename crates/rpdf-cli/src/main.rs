use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "rpdf", about = "PDF 파일 디버깅 도구")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 문서 메타데이터와 페이지 수를 출력한다.
    Info {
        /// PDF 파일 경로.
        #[arg(value_name = "PDF")]
        file: PathBuf,
        /// JSON 형식으로 출력.
        #[arg(long)]
        json: bool,
    },
    /// 페이지 메타데이터 목록을 출력한다 (MediaBox, Rotate, 연산자 수).
    #[command(name = "dump-pages")]
    DumpPages {
        /// PDF 파일 경로.
        #[arg(value_name = "PDF")]
        file: PathBuf,
        /// 출력할 페이지 인덱스 (0-based, 예: 0은 첫 번째 페이지). 미지정 시 전체 페이지.
        #[arg(short = 'p', long = "page", value_name = "PAGE")]
        page: Option<usize>,
        /// JSON 형식으로 출력.
        #[arg(long)]
        json: bool,
    },
    /// content stream 연산자 시퀀스를 출력한다 (PDF 키워드: BT, ET, Tj 등).
    Dump {
        /// PDF 파일 경로.
        #[arg(value_name = "PDF")]
        file: PathBuf,
        /// 출력할 페이지 인덱스 (0-based, 예: 0은 첫 번째 페이지). 미지정 시 전체 페이지.
        #[arg(short = 'p', long = "page", value_name = "PAGE")]
        page: Option<usize>,
        /// JSON 형식으로 출력.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Info { file, json } => {
            let data = read_file(&file)?;
            commands::info::run(&data, json)
        }
        Commands::DumpPages { file, page, json } => {
            let data = read_file(&file)?;
            commands::dump_pages::run(&data, page, json)
        }
        Commands::Dump { file, page, json } => {
            let data = read_file(&file)?;
            commands::dump::run(&data, page, json)
        }
    }
}

fn read_file(path: &PathBuf) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("파일을 읽을 수 없습니다: {}", path.display()))
}
