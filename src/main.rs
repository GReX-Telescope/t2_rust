use color_eyre::eyre::Result;
use linfa::traits::*;
use linfa_clustering::Dbscan;
use ndarray::prelude::*;
use std::collections::HashMap;
use tokio::net::UdpSocket;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Debug, Copy, Clone)]
struct Candidate {
    snr: f32,
    _f_n: i32,
    time_n: i32,
    mjds: f32,
    box_n: i32,
    dm_n: i32,
    dm: f32,
}

impl Candidate {
    fn from_str(cap: &str) -> Self {
        let splits: Vec<_> = cap.trim().split('\t').collect();
        Self {
            snr: splits[0].parse().unwrap(),
            _f_n: splits[1].parse().unwrap(),
            time_n: splits[2].parse().unwrap(),
            mjds: splits[3].parse().unwrap(),
            box_n: splits[4].parse().unwrap(),
            dm_n: splits[5].parse().unwrap(),
            dm: splits[6].parse().unwrap(),
        }
    }

    async fn insert(&self, pool: &sqlx::PgPool) -> Result<()> {
        let query = "INSERT INTO t2_cands (mjds, snr, ibox, dm) VALUES ($1, $2, $3, $4)";
        sqlx::query(query)
            .bind(self.mjds)
            .bind(self.snr)
            .bind(self.box_n)
            .bind(self.dm)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Cluster candidates in time, dm, box width space
fn cluster_params(cands: &[Candidate]) -> Array2<f64> {
    let mut params = Array::zeros((0, 3));
    for cand in cands {
        params
            .push_row(ArrayView::from(&[
                cand.time_n as f64,
                cand.dm_n as f64,
                (cand.box_n as f64).log2(),
            ]))
            .unwrap();
    }
    params
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // Create the socket to receive candidates from heimdall
    let socket = UdpSocket::bind("127.0.0.1:12345").await?;

    let mut buf = [0; 512];

    // Filter params
    // TODO - make these launch args
    let min_dm = 20.0;
    let max_dm = 100.0;
    let min_snr = 20.0;

    // Setup SQL connection to GReX database
    let url = "postgres://postgres:password@localhost:5432/grex";
    let pool = sqlx::postgres::PgPool::connect(url).await?;

    // Setup table
    sqlx::migrate!("./migrations").run(&pool).await?;

    let mut cands = Vec::new();

    loop {
        loop {
            let (n, _) = socket.recv_from(&mut buf).await?;
            if (n == 1) && (buf[0] == 0x03) {
                break;
            }
            let cand = Candidate::from_str(std::str::from_utf8(&buf[..n]).unwrap());
            cands.push(cand)
        }
        info!("Clustering glup of size - {}", cands.len());

        if cands.is_empty() {
            continue;
        }

        // Cluster (get idxs)
        let mut clusters: HashMap<usize, Candidate> = HashMap::new();
        let cluster_points = cluster_params(&cands);
        let cluster_idxs = Dbscan::params(5)
            .tolerance(14.0)
            .transform(&cluster_points)?;

        for (cand_i, maybe_cluster) in cluster_idxs.iter().enumerate() {
            let cand = cands[cand_i];
            if let Some(cluster_i) = maybe_cluster {
                if let Some(current_cluster_cand) = clusters.get(cluster_i) {
                    // Replace if the SNR is larger
                    if cand.snr > current_cluster_cand.snr {
                        clusters.insert(*cluster_i, cand);
                    }
                } else {
                    clusters.insert(*cluster_i, cand);
                }
            }
        }
        // Now flatten the candidates and remove the ones we don't care about
        let filtered: Vec<_> = clusters
            .into_values()
            .filter(|cand| cand.snr > min_snr)
            .filter(|cand| cand.dm > min_dm && cand.dm < max_dm)
            .collect();

        // Write the filtered, clustered candidates to the database
        info!("Writting {} candidates to database", filtered.len());
        for cand in filtered.iter() {
            cand.insert(&pool).await?;
        }
        cands.clear();
    }
}
