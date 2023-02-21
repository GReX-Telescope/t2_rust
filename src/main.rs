use linfa::traits::*;
use linfa_clustering::Dbscan;
use ndarray::prelude::*;
use plotters::prelude::*;
use std::{collections::HashMap, net::UdpSocket};

#[derive(Debug, Copy, Clone)]
struct Candidate {
    snr: f64,
    _f_n: usize,
    time_n: usize,
    mjds: f64,
    box_n: usize,
    dm_n: usize,
    dm: f64,
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
}

fn cluster_params(cands: &[Candidate]) -> Array2<f64> {
    let mut params = Array::zeros((0, 3));
    for cand in cands {
        params
            .push_row(ArrayView::from(&[
                cand.time_n as f64,
                cand.dm_n as f64,
                cand.box_n as f64,
            ]))
            .unwrap();
    }
    params
}

fn main() -> anyhow::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:12345")?;

    let mut buf = [0; 512];
    let min_dm = 20.0;
    let max_dm = 100.0;
    let min_snr = 20.0;

    let mut count = 0;

    loop {
        let mut cands = Vec::new();
        while cands.len() < 1024 {
            let (n, _) = socket.recv_from(&mut buf)?;
            let cand = Candidate::from_str(std::str::from_utf8(&buf[..n]).unwrap());
            cands.push(cand)
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
        // Plot the clusters
        let filename = format!("target/{count}.png");
        let root = BitMapBackend::new(&filename, (600, 400)).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let x_lim = min_dm..max_dm;
        let y_lim = 0.0..1000.0;

        let mut ctx = ChartBuilder::on(&root)
            .set_label_area_size(LabelAreaPosition::Left, 40) // Put in some margins
            .set_label_area_size(LabelAreaPosition::Right, 40)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .caption("Clustering", ("sans-serif", 25)) // Set a caption and font
            .build_cartesian_2d(x_lim, y_lim)
            .expect("Couldn't build our ChartBuilder");

        ctx.configure_mesh().draw().unwrap();
        let root_area = ctx.plotting_area();

        // We're only going to plot the first two dims
        for i in 0..cluster_points.shape()[0] {
            let cand = cands[i];

            let point = match cluster_idxs[i] {
                Some(0) => Circle::new((cand.dm, cand.mjds), 3, ShapeStyle::from(&RED).filled()),
                Some(1) => Circle::new((cand.dm, cand.mjds), 3, ShapeStyle::from(&GREEN).filled()),
                Some(2) => Circle::new((cand.dm, cand.mjds), 3, ShapeStyle::from(&BLUE).filled()),
                // Making sure our pattern-matching is exhaustive
                // Note that we can define a custom color using RGB
                _ => Circle::new(
                    (cand.dm, cand.mjds),
                    3,
                    ShapeStyle::from(&RGBColor(255, 255, 255)).filled(),
                ),
            };

            root_area
                .draw(&point)
                .expect("An error occurred while drawing the point!");
        }

        dbg!(filtered);
        cands.clear();
        count += 1;
    }
}
