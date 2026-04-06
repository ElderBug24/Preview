use std::env;
use std::io::{self, Write};
use std::fmt::Write as Write_;
use std::path::PathBuf;

use braille::BrailleChar;

use glam::Vec3;
use image::{imageops::FilterType, GenericImageView, SubImage, Rgb, Rgb32FImage, ImageResult};
use crossterm::terminal;


struct Buffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Vec3>,
}

impl Buffer {
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Vec3 {
        return self.data[y * self.width + x];
    }

    #[inline]
    fn get_mut(&mut self, x: usize, y: usize) -> &mut Vec3 {
        return &mut self.data[y * self.width + x];
    }

    #[inline]
    pub fn from_file(img: Rgb32FImage) -> Self {
        let (w, h) = img.dimensions();
        let (w, h) = (w as usize, h as usize);

        let flat = img.into_flat_samples();
        let bytes = flat.samples;
        let mut data = unsafe { std::mem::transmute::<Vec<f32>, Vec<Vec3>>(bytes) };
        unsafe { data.set_len(w * h) };

        return Self {
            width: w,
            height: h,
            data: data
        };
    }
}

fn average_color(sub: &SubImage<&image::RgbImage>) -> (u8, u8, u8) {
    let mut sum_r = 0u16;
    let mut sum_g = 0u16;
    let mut sum_b = 0u16;
    let mut count = 0u16;

    for (_, _, pixel) in sub.pixels() {
        let Rgb([r, g, b]) = pixel;
        sum_r += r as u16;
        sum_g += g as u16;
        sum_b += b as u16;
        count += 1;
    }

    return (
        (sum_r / count) as u8,
        (sum_g / count) as u8,
        (sum_b / count) as u8,
    );
}

const HELP: &str = "\
Preview any image file
Usage: preview [OPTIONS]
Options:
    -c, --color                       Colorize the image using ANSI escape codes
    -C, --color-only                  Colorize the image using ANSI escape codes, replacing all characters with ⣿
    -h, --help                        Print this help message
    -v, --verbose                     Use verbose output
    [FILENAME]                        Specify input filename
    [WIDTH]x[HEIGHT]                  Specify output dimensions
    [WIDTH]                           Specify output width; height is scaled proportionally";

fn main() {
    let args = env::args().skip(1);

    let mut width = 0;
    let mut height = 0;

    let mut path = None;
    let mut color = 0;
    let mut verbose = false;
    let mut help = false;

    for arg in args {
        if let Some((a, b)) = arg.split_once('x') {
            match (a.parse::<u32>(), b.parse::<u32>()) {
                (Ok(w), Ok(h)) => {
                    width = w;
                    height = h;
                }
                _ => {
                    let mut p = PathBuf::new();
                    p.push(&arg);

                    if p.is_file() {
                        path = Some(p);
                    } else {
                        println!("Error: '{}' is invalid", arg);
                    }
                },
            }
        } else if let Ok(w) = arg.parse::<u32>() {
            width = w;
        } else if arg == "-c" || arg == "--color" {
            color = 1;
        } else if arg == "-C" || arg == "--color-only" {
            color = 2;
        } else if arg == "-v" || arg == "--verbose" {
            verbose = true;
        } else if arg == "-h" || arg == "--help" {
            help = true;
        } else {
            let mut p = PathBuf::new();
            p.push(&arg);

            if p.is_file() {
                path = Some(p);
            } else {
                println!("Error: '{}' is invalid", arg);
            }
        }
    }

    if help {
        println!("{}", HELP);

        return;
    }

    let mut img = match path {
        Some(path) => match image::open(&path) {
            ImageResult::Ok(img) => img,
            ImageResult::Err(error) => {
                println!("Error: {}", error);

                return;
            }
        },
        None => {
            println!("Error: Missing input file");

            return;
        }
    };
    let (w, h) = img.dimensions();

    if width == 0 && height == 0 {
        if let Ok((cols, rows)) = terminal::size() {
            let rows = (rows - 2) * 2;

            if (rows as i32) - (h as i32) < (cols as i32) - (w as i32) {
                height = rows as u32;
                width = height * w / h;
            } else {
                width = cols as u32;
                height = width * h / w;
            }
        }
    }
    if height == 0 {
        if width == 0 {
            width = 80;
        }
        height = width * h / w;
    }

    let mut out = String::with_capacity((height / 2 * (width + 1)) as usize);

    if color != 2 {
        let w_ = (width * 2) as usize;
        let h_ = (height * 2) as usize;

        img = img.resize_exact(w_ as u32, h_ as u32, FilterType::Nearest);
        let img2 = img.to_rgb8();

        let mut buffer = Buffer::from_file(img.to_rgb32f());

        for i in 0..(h_/4) {
            for j in 0..(w_/2) {
                let mut buf = [false; 8];
                for k in 0..8 {
                    let x = k % 2 + (8 * j + k) / 8 * 2;
                    let y = i * 4 + (8 * j + k) % 8 / 2;

                    let oldpixel = buffer.get(x, y).saturate();

                    let (b, nl) = match oldpixel.element_sum() {
                        0.0..1.5 => (false, 0.0),
                        _ => (true, 1.0)
                    };

                    buf[k] = b;

                    let quant_error = (oldpixel - nl) / 8.0;

                    let right = x + 1 < buffer.width;
                    let right2 = x + 2 < buffer.width;
                    let left = x > 0;
                    let down = y + 1 < buffer.height;
                    let down2 = y + 2 < buffer.height;

                    if right {
                        *buffer.get_mut(x+1, y) += quant_error;
                        if right2 {
                            *buffer.get_mut(x+2, y) += quant_error;
                        }
                        if down {
                            *buffer.get_mut(x+1, y+1) += quant_error;
                        }
                    }
                    if down {
                        *buffer.get_mut(x, y+1) += quant_error;
                        if left {
                            *buffer.get_mut(x-1, y+1) += quant_error;
                        }
                        if down2 {
                            *buffer.get_mut(x, y+2) += quant_error;
                        }
                    }
                }

                let char = BrailleChar::from_array_unordered(&buf);

                if color != 0 {
                    let view = img2.view(j as u32 * 2, i as u32 * 4, 2, 4);
                    let (r, g, b) = average_color(&view);

                    write!(out, "\x1b[38;2;{};{};{}m", r, g, b).unwrap()
                }
                out.push(char.char());
            }
            out.push_str("\x1b[0m\n");
        }
    } else {
        let img = img.resize_exact(width as u32, height as u32 / 2, FilterType::Triangle).into_rgb8();

        for y in 0..(height / 2) {
            for x in 0..width {
                let [r, g, b] = img.get_pixel(x as u32, y as u32).0;

                write!(out, "\x1b[48;2;{};{};{}m{}", r, g, b, ' ').unwrap();
            }
            out.push_str("\x1b[0m\n");
        }
    }

    if verbose {
        write!(out, "Resized image from {}x{} to {}x{}", w, h, width, height).unwrap();
        match color {
            1 => out.push_str(" (color)"),
            2 => out.push_str(" (color-only)"),
            _ => {}
        }
    }

    io::stdout().lock().write_all(out.as_bytes()).unwrap();
}

