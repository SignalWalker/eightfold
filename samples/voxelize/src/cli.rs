use std::path::PathBuf;

use clap::ValueHint;
use nalgebra::{Scalar, Scale3, Vector3};

use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, clap::ValueEnum)]
pub enum LogFormat {
    Compact,
    Full,
    Pretty,
    Json,
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Compact => f.write_str("compact"),
            LogFormat::Full => f.write_str("full"),
            LogFormat::Pretty => f.write_str("pretty"),
            LogFormat::Json => f.write_str("json"),
        }
    }
}

#[derive(Debug, clap::Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// Logging output filters; comma-separated
    #[arg(
        short,
        long,
        default_value = "warn,eightfold=info,voxelize=info",
        env = "VOXELIZE_LOG_FILTER"
    )]
    pub log_filter: String,
    /// Logging output format
    #[arg(long, default_value_t = LogFormat::Pretty)]
    pub log_format: LogFormat,
    /// The unitless size of a single voxel
    #[arg(short, long, default_value = "1,1,1", value_parser = parse_vec3::<f32>, value_name = "X,Y,Z")]
    pub voxel_size: Vector3<f32>,
    /// Scaling applied to each mesh before processing
    #[arg(short, long, default_value = "1,1,1", value_parser = parse_scale3::<f32>, value_name = "X,Y,Z")]
    pub mesh_scale: Scale3<f32>,
    /// Files to voxelize
    #[arg(num_args = 1.., required = true, value_hint = ValueHint::FilePath)]
    pub files: Vec<PathBuf>,
}

fn parse_vec3<R: FromStr>(
    s: &str,
) -> Result<Vector3<R>, Box<dyn std::error::Error + Send + Sync + 'static>>
where
    <R as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    let mut split = s.trim().split(',');
    let x = R::from_str(split.next().unwrap())?;
    let y = R::from_str(split.next().unwrap())?;
    let z = R::from_str(split.next().unwrap())?;
    Ok(nalgebra::vector![x, y, z])
}

fn parse_scale3<R: FromStr + Scalar>(
    s: &str,
) -> Result<Scale3<R>, Box<dyn std::error::Error + Send + Sync + 'static>>
where
    <R as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    parse_vec3::<R>(s).map(Scale3::from)
}

/// Set up pretty log output
pub(crate) fn initialize_tracing(log_filter: &str, log_format: LogFormat) {
    let tsub = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_timer(tracing_subscriber::fmt::time::OffsetTime::new(
            time::UtcOffset::current_local_offset().unwrap_or_else(|e| {
                tracing::warn!("couldn't get local time offset: {:?}", e);
                time::UtcOffset::UTC
            }),
            time::macros::format_description!("[hour]:[minute]:[second]"),
        ))
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_env_filter(log_filter);

    match log_format {
        LogFormat::Compact => tsub.compact().init(),
        LogFormat::Full => tsub.init(),
        LogFormat::Pretty => tsub.pretty().init(),
        LogFormat::Json => tsub.json().init(),
    }
}

/// Convenience macro for emitting [tracing] events about [Accessors](gltf::Accessor).
#[macro_export]
macro_rules! trace_accessor {
    ($acc:ident, $msg:expr) => {
        tracing::trace!(
            index = $acc.index(),
            data_type = format!("{:?}", $acc.data_type(),),
            dimensions = format!("{:?}", $acc.dimensions(),),
            count = $acc.count(),
            name = $acc.name(),
            $msg
        );
    };
}
