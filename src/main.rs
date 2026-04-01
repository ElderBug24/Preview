use std::env;
use std::io::{self, Write};
use std::fmt::Write as Write_;
use std::path::PathBuf;

use braille::{BrailleCharUnOrdered, BrailleCharGridVector};

use glam::Vec3;
use image::{imageops::FilterType, GenericImageView, SubImage, Rgb};
use owo_colors::OwoColorize;

struct Buffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Vec3>,
}

impl Buffer {
    pub fn _new(width: usize, height: usize) -> Self {
        return Self {
            width: width,
            height: height,
            data: vec![Vec3::ZERO; width * height]
        };
    }

    pub fn dimensions(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Vec3 {
        return self.data[y * self.width + x];
    }

    #[inline]
    pub fn _set(&mut self, x: usize, y: usize, value: Vec3) {
        self.data[y * self.width + x] = value;
    }

    #[inline]
    fn get_mut(&mut self, x: usize, y: usize) -> &mut Vec3 {
        return &mut self.data[y * self.width + x];
    }

    pub fn from_file(img: image::DynamicImage, w: u32, h: u32) -> Self {
        let img = img.resize_exact(w, h, FilterType::Triangle).into_rgb32f();

        let (w, h) = img.dimensions();
        let (w, h) = (w as usize, h as usize);

        let flat = img.as_flat_samples();
        let bytes = flat.samples;
        let mut data = unsafe { std::mem::transmute::<Vec<f32>, Vec<Vec3>>(bytes.to_vec()) };
        unsafe { data.set_len(w * h) };

        let buf = Self {
            width: w,
            height: h,
            data: data
        };

        return buf;
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
    -b, --blur-color [VALUE]          Blurs the image's colors; value controls image flattening level
    -B, --blur [VALUE]                Blurs the image; value controls image flattening level
    -h, --help                        Print this help message
    -v, --verbose                     Use verbose output
    [FILENAME]                        Specify input filename
    [WIDTH]x[HEIGHT]                  Specify output dimensions
    [WIDTH]                           Specify output width; height is scaled proportionally";

enum ExpectedArgument {
    None,
    Blur,
    BlurColor
}

fn main() {
    let args = env::args().skip(1);

    let mut width = 0;
    let mut height = 0;

    let mut path = None;
    let mut color = 0;
    let mut blur = 0;
    let mut blur_color = 0;
    let mut verbose = false;
    let mut help = false;

    let mut expected = ExpectedArgument::None;

    for arg in args {
        match expected {
                ExpectedArgument::None => {
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
                } else if arg == "-B" || arg == "--blur" {
                    expected = ExpectedArgument::Blur;
                } else if arg == "-b" || arg == "--blur-color" {
                    expected = ExpectedArgument::BlurColor;
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
            },
            ExpectedArgument::Blur => {
                match arg.parse::<u8>() {
                    Ok(sigma) => blur = sigma,
                    Err(e) => eprintln!("{} {}", e, arg)
                }
                expected = ExpectedArgument::None;
            },
            ExpectedArgument::BlurColor => {
                match arg.parse::<u8>() {
                    Ok(sigma) => blur_color = sigma,
                    Err(e) => eprintln!("{} {}", e, arg)
                }
                expected = ExpectedArgument::None;
            }
        };
    }

    if help {
        println!("{}", HELP);

        return;
    }

    let mut img = image::open(&path.expect("Error: Missing input file")).expect("Error: Could not open file");
    let (w, h) = img.dimensions();

    if height == 0 {
        if width == 0 {
            width = 160;
        }
        height = h * width / w;
    }

    let mut img2 = img.resize_exact(width, height, FilterType::Nearest);

    if blur_color == 0 {
        blur_color = blur;
    }
    if blur > 0 {
        img = img.fast_blur(blur as f32);
    }
    if blur_color > 0 {
        img2 = img2.fast_blur(blur_color as f32);
    }
    let img2 = img2.to_rgb8();

    let mut buffer = Buffer::from_file(img, width, height);

    let (w, h) = buffer.dimensions();

    let w_ = w/2*2;
    let h_ = h/4*4;

    let mut array = vec![false; w_ * h_];
    let mut grid: BrailleCharGridVector<BrailleCharUnOrdered> = BrailleCharGridVector::new(w/2, h/4);

    let mut out = String::with_capacity(h/4 * (w/2 + 1));

    for y_ in 0..(h/4) {
        for y__ in 0..4 {
            let y = y_ * 4 + y__;
            for x in 0..w_ {
                let oldpixel = buffer.get(x, y).clamp(Vec3::ZERO, Vec3::splat(1.0));

                let (b, nl) = match oldpixel.element_sum() {
                    0.0..1.5 => (false, 0.0),
                    _ => (true, 1.0)
                };

                grid.set_unchecked(x, y, b);

                array[x + y * w_] = b;

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
        }

        for x in 0..(w/2) {
            let arr = [
                array[(2 * x) + 4 * y_ * w_],
                array[(2 * x) + 1 + 4 * y_ * w_],
                array[(2 * x) + (4 * y_ + 1) * w_],
                array[(2 * x) + 1 + (4 * y_ + 1) * w_],
                array[(2 * x) + (4 * y_ + 2) * w_],
                array[(2 * x) + 1 + (4 * y_ + 2) * w_],
                array[(2 * x) + (4 * y_ + 3) * w_],
                array[(2 * x) + 1 + (4 * y_ + 3) * w_],
            ];

            let char = BrailleCharUnOrdered::from_array_unordered(arr);

            match color {
                0 => write!(out, "{}", char.char()).unwrap(),
                1 => {
                    let view = img2.view(x as u32 * 2, y_ as u32 * 4, 2, 4);
                    let (r, g, b) = average_color(&view);

                    write!(out, "{}", char.char().truecolor(r, g, b)).unwrap();
                },
                2 => {
                    let view = img2.view(x as u32 * 2, y_ as u32 * 4, 2, 4);
                    let (r, g, b) = average_color(&view);

                    write!(out, "{}", BrailleCharUnOrdered::FULL.char().truecolor(r, g, b)).unwrap();
                },
                _ => unreachable!()
            }
        }
        out.push('\n');
    }

    if verbose {
        if color < 2 {
            write!(out, "Resizing from {} x {} to {} x {}", w, h, width, height).unwrap();
            if color > 0 {
                out.push_str("\n + Coloring");
            }
        } else {
            write!(out, "The color from the image was rendered at a size of {} x {}", width, height).unwrap();
        }
        if blur > 0 {
            write!(out, "\n + Blurring: {}", blur).unwrap();
        }
        if blur_color > 0 {
            write!(out, "\n + Blurring color {}", blur_color).unwrap();
        }
    }

    io::stdout().lock().write_all(out.as_bytes()).unwrap();
}

