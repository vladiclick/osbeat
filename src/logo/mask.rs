//! Optional pixel coverage, independent of the logo or its animation.
use std::io;

#[derive(Debug)]
pub struct Mask {
    width: usize,
    height: usize,
    alpha: Vec<u8>,
}

impl Mask {
    pub(super) fn parse(data: &str) -> io::Result<Self> {
        let rows: Vec<&str> = data.lines().collect();
        let width = rows.first().map_or(0, |row| row.len());
        if width == 0 || width > 512 || rows.len() > 256 || rows.iter().any(|r| r.len() != width) {
            return Err(super::invalid(
                "mask must be a rectangular grid, at most 512 × 256",
            ));
        }
        let alpha = rows
            .iter()
            .flat_map(|r| r.bytes())
            .map(|ch| {
                (ch as char)
                    .to_digit(16)
                    .map(|n| n as u8 * 17)
                    .ok_or_else(|| super::invalid("mask coverage must use hexadecimal digits 0–f"))
            })
            .collect::<io::Result<Vec<_>>>()?;
        if alpha.iter().all(|&a| a == 0) {
            return Err(super::invalid("mask must contain visible pixels"));
        }
        Ok(Self {
            width,
            height: rows.len(),
            alpha,
        })
    }

    pub(crate) fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Area filtering preserves a thin leaf or small hole when a terminal shrinks.
    /// It runs only when rebuilding geometry, never on animation frames.
    pub(crate) fn coverage(&self, x: usize, y: usize, width: usize, height: usize) -> u8 {
        let x0 = x as f32 * self.width as f32 / width as f32;
        let x1 = (x + 1) as f32 * self.width as f32 / width as f32;
        let y0 = y as f32 * self.height as f32 / height as f32;
        let y1 = (y + 1) as f32 * self.height as f32 / height as f32;
        let mut sum = 0.0;
        for sy in y0.floor() as usize..(y1.ceil() as usize).min(self.height) {
            let dy = (y1.min((sy + 1) as f32) - y0.max(sy as f32)).max(0.0);
            for sx in x0.floor() as usize..(x1.ceil() as usize).min(self.width) {
                let dx = (x1.min((sx + 1) as f32) - x0.max(sx as f32)).max(0.0);
                sum += f32::from(self.alpha[sy * self.width + sx]) * dx * dy;
            }
        }
        (sum / ((x1 - x0) * (y1 - y0))).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::Mask;

    #[test]
    fn area_sampling_preserves_transparency_and_averages_edges() {
        let mask = Mask::parse("f0\n08").unwrap();
        assert_eq!(mask.coverage(0, 0, 2, 2), 255);
        assert_eq!(mask.coverage(1, 0, 2, 2), 0);
        assert_eq!(mask.coverage(1, 1, 2, 2), 136);
        assert_eq!(mask.coverage(0, 0, 1, 1), 98);
        assert_eq!(mask.coverage(0, 0, 4, 4), 255);
        assert_eq!(mask.coverage(3, 3, 4, 4), 136);
        assert_eq!(mask.aspect(), 1.0);
    }

    #[test]
    fn bundled_apple_preserves_leaf_gap_bite_and_bottom_notch() {
        let logo = crate::logo::Logo::for_os(crate::model::Os::Mac).unwrap();
        let mask = logo.mask.unwrap();
        assert!(mask.coverage(56, 10, 96, 96) > 200, "leaf");
        assert!(mask.coverage(54, 23, 96, 96) < 20, "leaf/body gap");
        assert!(mask.coverage(84, 48, 96, 96) < 20, "bite");
        assert!(mask.coverage(16, 48, 96, 96) > 200, "body");
        assert!(mask.coverage(48, 94, 96, 96) < 20, "bottom notch");
    }
}
