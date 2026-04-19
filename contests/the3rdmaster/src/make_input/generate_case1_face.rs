use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;

const N: usize = 32;
const K: usize = 5;
const C: usize = 4;

#[derive(Clone, Copy)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

fn run_ffmpeg_rgb32x32(input_path: &Path) -> Result<Vec<Rgb>, String> {
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-i",
            input_path
                .to_str()
                .ok_or_else(|| "input path is not valid UTF-8".to_owned())?,
            "-vf",
            "scale=32:32:flags=lanczos",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg failed (status={}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let expected = N * N * 3;
    if output.stdout.len() != expected {
        return Err(format!(
            "unexpected raw size: got {}, expected {}",
            output.stdout.len(),
            expected
        ));
    }

    let mut pixels = Vec::with_capacity(N * N);
    for chunk in output.stdout.chunks_exact(3) {
        pixels.push(Rgb {
            r: chunk[0],
            g: chunk[1],
            b: chunk[2],
        });
    }
    Ok(pixels)
}

fn is_background(px: Rgb, threshold: u8) -> bool {
    px.r >= threshold && px.g >= threshold && px.b >= threshold
}

fn luminance(px: Rgb) -> f64 {
    0.299 * px.r as f64 + 0.587 * px.g as f64 + 0.114 * px.b as f64
}

fn choose_background_threshold(pixels: &[Rgb]) -> u8 {
    let mut chosen = 255u8;
    for t in (200u8..=255u8).rev() {
        let nonzero = pixels.iter().filter(|&&px| !is_background(px, t)).count();
        if nonzero >= (N * N) / 2 {
            chosen = t;
        } else {
            break;
        }
    }
    chosen
}

fn quantile(sorted: &[f64], num: usize, den: usize) -> f64 {
    let idx = ((sorted.len().saturating_sub(1)) * num) / den;
    sorted[idx]
}

fn build_grid(pixels: &[Rgb], bg_threshold: u8) -> (Vec<Vec<usize>>, usize) {
    let mut ys = Vec::with_capacity(pixels.len());
    for &px in pixels {
        if !is_background(px, bg_threshold) {
            ys.push(luminance(px));
        }
    }
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let q1 = if ys.is_empty() { 64.0 } else { quantile(&ys, 1, 3) };
    let q2 = if ys.is_empty() { 128.0 } else { quantile(&ys, 2, 3) };

    let mut g = vec![vec![0usize; N]; N];
    let mut nonzero = 0usize;
    for i in 0..N {
        for j in 0..N {
            let px = pixels[i * N + j];
            let v = if is_background(px, bg_threshold) {
                0
            } else {
                let y = luminance(px);
                if y < q1 {
                    1
                } else if y < q2 {
                    2
                } else {
                    4
                }
            };
            if v != 0 {
                nonzero += 1;
            }
            g[i][j] = v;
        }
    }
    (g, nonzero)
}

fn write_input(path: &Path, g: &[Vec<usize>]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("failed to create input file: {e}"))?;
    let mut w = BufWriter::new(file);
    writeln!(w, "{N} {K} {C}").map_err(|e| e.to_string())?;
    for row in g {
        for (j, &v) in row.iter().enumerate() {
            if j > 0 {
                write!(w, " ").map_err(|e| e.to_string())?;
            }
            write!(w, "{v}").map_err(|e| e.to_string())?;
        }
        writeln!(w).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn write_output(path: &Path, g: &[Vec<usize>]) -> Result<usize, String> {
    let file = File::create(path).map_err(|e| format!("failed to create output file: {e}"))?;
    let mut w = BufWriter::new(file);
    let mut actions = 0usize;
    for i in 0..N {
        for j in 0..N {
            let v = g[i][j];
            if v == 0 {
                continue;
            }
            writeln!(w, "0 0 {i} {j} {v}").map_err(|e| e.to_string())?;
            actions += 1;
        }
    }
    Ok(actions)
}

fn vis_rgb(v: usize) -> (u8, u8, u8) {
    match v {
        0 => (0xF8, 0xFA, 0xFC),
        1 => (0x25, 0x63, 0xEB),
        2 => (0xEF, 0x44, 0x44),
        3 => (0x10, 0xB9, 0x81),
        4 => (0xF5, 0x9E, 0x0B),
        _ => (0x6B, 0x72, 0x80),
    }
}

fn write_preview_ppm(path: &Path, g: &[Vec<usize>]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("failed to create preview file: {e}"))?;
    let mut w = BufWriter::new(file);
    writeln!(w, "P3").map_err(|e| e.to_string())?;
    writeln!(w, "{} {}", N, N).map_err(|e| e.to_string())?;
    writeln!(w, "255").map_err(|e| e.to_string())?;
    for i in 0..N {
        for j in 0..N {
            let (r, gch, b) = vis_rgb(g[i][j]);
            writeln!(w, "{r} {gch} {b}").map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let base = Path::new("src/make_input");
    let src = base.join("takahashi_face.png");
    if !src.exists() {
        return Err(format!("source image not found: {}", src.display()));
    }

    let pixels = run_ffmpeg_rgb32x32(&src)?;
    let bg_threshold = choose_background_threshold(&pixels);
    let (grid, nonzero) = build_grid(&pixels, bg_threshold);

    if nonzero < (N * N) / 2 {
        return Err(format!(
            "non-zero pixels too few: {nonzero} < {}",
            (N * N) / 2
        ));
    }

    let input_path = base.join("case1_face_input.txt");
    let output_path = base.join("case1_face_output.txt");
    let preview_ppm_path = base.join("case1_face_preview.ppm");
    let preview_png_path = base.join("case1_face_preview.png");

    write_input(&input_path, &grid)?;
    let actions = write_output(&output_path, &grid)?;
    write_preview_ppm(&preview_ppm_path, &grid)?;

    let png_conv = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-i",
            preview_ppm_path
                .to_str()
                .ok_or_else(|| "preview ppm path is not valid UTF-8".to_owned())?,
            preview_png_path
                .to_str()
                .ok_or_else(|| "preview png path is not valid UTF-8".to_owned())?,
        ])
        .status()
        .map_err(|e| format!("failed to run ffmpeg for preview png: {e}"))?;

    if !png_conv.success() {
        return Err(format!("ffmpeg conversion failed: status={png_conv}"));
    }

    println!("generated: {}", input_path.display());
    println!("generated: {}", output_path.display());
    println!("generated: {}", preview_ppm_path.display());
    println!("generated: {}", preview_png_path.display());
    println!("bg_threshold={bg_threshold}, nonzero={nonzero}, actions={actions}");
    Ok(())
}
