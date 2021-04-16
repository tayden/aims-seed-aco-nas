#[macro_use]
extern crate dotenv_codegen;

use std::collections::HashSet;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::{ffi, fs, io};

use log::{debug, error};
use postgres::types::Type;
use postgres::{Client, NoTls};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// Create ACO directory seeds on the NAS using the AIMS database projects
pub struct Cli {
    /// The year in the AIMS to search for projects
    pub year: i32,

    /// The minimum project status to seed
    pub min_status: String,

    /// The root directory where project directories are located
    #[structopt(parse(from_os_str = Self::parse_canonical_path))]
    pub root_dir: PathBuf,

    /// The seed directory to copy to produce a new empty project directory under the root_dir
    #[structopt(parse(from_os_str = Self::parse_canonical_path))]
    pub seed_dir: PathBuf,
}

impl Cli {
    fn parse_canonical_path(path: &ffi::OsStr) -> PathBuf {
        let buf = PathBuf::from(path);
        match fs::canonicalize(&buf) {
            Ok(path) => path,
            Err(e) => {
                error!("Could not parse system path {:?}: {}", &buf, &e);
                panic!("{}", e)
            }
        }
    }
}

fn get_db_client() -> Result<Client, postgres::Error> {
    let connection = format!(
        "host={} port={} dbname={} user={} password={}",
        dotenv!("DB_HOST"),
        dotenv!("DB_PORT"),
        dotenv!("DB_NAME"),
        dotenv!("DB_USER"),
        dotenv!("DB_PASS")
    );
    debug!("Postgres connection: {}", &connection);

    Client::connect(&connection, NoTls)
}

pub fn get_db_projects(
    root_dir: &PathBuf,
    year: &i32,
    min_status: &String,
) -> Result<HashSet<PathBuf>, postgres::Error> {
    let mut client = get_db_client()?;

    let stmt = client.prepare_typed(
        "
        SELECT dirname 
            FROM aco.output_project_phases
            WHERE project_year = $1
            AND status_project >= $2::aco.enum_status_phase
        ",
        &[Type::INT4, Type::TEXT],
    )?;

    let paths: Vec<PathBuf> = client
        .query(&stmt, &[&year, &min_status])?
        .into_iter()
        .map(|row| row.get(0))
        .map(|r: String| root_dir.join(r))
        .collect();

    Ok(HashSet::<PathBuf>::from_iter(paths))
}

pub fn get_fs_projects(root_dir: &PathBuf) -> Result<HashSet<PathBuf>, io::Error> {
    let directories: Vec<PathBuf> = fs::read_dir(&root_dir)?
        .into_iter()
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap().path())
        .filter(|r| r.is_dir())
        .collect();

    Ok(HashSet::<PathBuf>::from_iter(directories))
}
