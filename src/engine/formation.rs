use std::f64::consts::TAU;

pub struct Formation;

impl Formation {
    pub fn positions(
        kind: &str,
        count: usize,
        center: (f64, f64),
        spacing: f64,
    ) -> Vec<(f64, f64)> {
        match kind {
            "line" => (0..count)
                .map(|i| {
                    (
                        center.0 + Self::centered_offset(i, count) * spacing,
                        center.1,
                    )
                })
                .collect(),
            "column" => (0..count)
                .map(|i| {
                    (
                        center.0,
                        center.1 + Self::centered_offset(i, count) * spacing,
                    )
                })
                .collect(),
            "staggered" => (0..count)
                .map(|i| {
                    let row = i / 2;
                    let side = if i % 2 == 0 { -0.5 } else { 0.5 };
                    (
                        center.0 + side * spacing,
                        center.1 + Self::centered_offset(row, count.div_ceil(2)) * spacing,
                    )
                })
                .collect(),
            "wedge" => (0..count)
                .map(|i| {
                    if i == 0 {
                        return center;
                    }
                    let row = ((i + 1) as f64).sqrt().floor() as usize;
                    let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                    (
                        center.0 + side * row as f64 * spacing * 0.5,
                        center.1 + row as f64 * spacing,
                    )
                })
                .collect(),
            "circle" => (0..count)
                .map(|i| {
                    let angle = i as f64 / count.max(1) as f64 * TAU;
                    (
                        center.0 + angle.cos() * spacing,
                        center.1 + angle.sin() * spacing,
                    )
                })
                .collect(),
            _ => {
                let side = (count as f64).sqrt().ceil() as usize;
                let rows = count.div_ceil(side.max(1));
                (0..count)
                    .map(|i| {
                        let x = i % side;
                        let y = i / side;
                        (
                            center.0 + Self::centered_offset(x, side) * spacing,
                            center.1 + Self::centered_offset(y, rows) * spacing,
                        )
                    })
                    .collect()
            }
        }
    }

    fn centered_offset(index: usize, count: usize) -> f64 {
        if count <= 1 {
            0.0
        } else {
            index as f64 - (count.saturating_sub(1) as f64 * 0.5)
        }
    }
}
