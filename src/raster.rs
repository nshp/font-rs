// Copyright 2015 Google Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! An antialiased rasterizer for quadratic Beziers

#[cfg(not(feature = "std"))]
use core::intrinsics;

#[cfg(feature = "std")]
use std::intrinsics;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use accumulate::accumulate;
use geom::Point;

// TODO: sort out crate structure. Right now we want this when compiling raster as a binary,
// but need it commented out when compiling showttf
//mod geom;

pub struct Raster {
    w: usize,
    h: usize,
    a: Vec<f32>,
}

// TODO: is there a faster way? (investigate whether approx recip is good enough)
fn recip(x: f32) -> f32 {
    x.recip()
}

fn fsqrt32(f: f32) -> f32 {
    unsafe { intrinsics::sqrtf32(f) }
}

fn floor32(f: f32) -> f32 {
    unsafe { intrinsics::floorf32(f) }
}

fn ceil32(f: f32) -> f32 {
    unsafe { intrinsics::ceilf32(f) }
}

impl Raster {
    pub fn new(w: usize, h: usize) -> Raster {
        Raster {
            w: w,
            h: h,
            a: vec![0.0; w * h + 4],
        }
    }

    pub fn draw_line(&mut self, p0: &Point, p1: &Point) {
        //println!("draw_line {} {}", p0, p1);
        if p0.y == p1.y {
            return;
        }
        let (dir, p0, p1) = if p0.y < p1.y {
            (1.0, p0, p1)
        } else {
            (-1.0, p1, p0)
        };
        let dxdy = (p1.x - p0.x) / (p1.y - p0.y);
        let mut x = p0.x;
        let y0 = p0.y as usize; // note: implicit max of 0 because usize (TODO: really true?)
        if p0.y < 0.0 {
            x -= p0.y * dxdy;
        }
        let ymin = self.h.min(ceil32(p1.y) as usize);
        for y in y0..ymin {
            let linestart = y * self.w;
            let dy = ((y + 1) as f32).min(p1.y) - (y as f32).max(p0.y);
            let xnext = x + dxdy * dy;
            let d = dy * dir;
            let (x0, x1) = if x < xnext { (x, xnext) } else { (xnext, x) };
            let x0floor = floor32(x0);
            let x0i = x0floor as i32;
            let x1ceil = ceil32(x1);
            let x1i = x1ceil as i32;
            if x1i <= x0i + 1 {
                let xmf = 0.5 * (x + xnext) - x0floor;
                self.a[linestart + x0i as usize] += d - d * xmf;
                self.a[linestart + (x0i + 1) as usize] += d * xmf;
            } else {
                let s = recip(x1 - x0);
                let x0f = x0 - x0floor;
                let a0 = 0.5 * s * (1.0 - x0f) * (1.0 - x0f);
                let x1f = x1 - x1ceil + 1.0;
                let am = 0.5 * s * x1f * x1f;
                self.a[linestart + x0i as usize] += d * a0;
                if x1i == x0i + 2 {
                    self.a[linestart + (x0i + 1) as usize] += d * (1.0 - a0 - am);
                } else {
                    let a1 = s * (1.5 - x0f);
                    self.a[linestart + (x0i + 1) as usize] += d * (a1 - a0);
                    for xi in x0i + 2..x1i - 1 {
                        self.a[linestart + xi as usize] += d * s;
                    }
                    let a2 = a1 + (x1i - x0i - 3) as f32 * s;
                    self.a[linestart + (x1i - 1) as usize] += d * (1.0 - a2 - am);
                }
                self.a[linestart + x1i as usize] += d * am;
            }
            x = xnext;
        }
    }

    pub fn draw_quad(&mut self, p0: &Point, p1: &Point, p2: &Point) {
        //println!("draw_quad {} {} {}", p0, p1, p2);
        let devx = p0.x - 2.0 * p1.x + p2.x;
        let devy = p0.y - 2.0 * p1.y + p2.y;
        let devsq = devx * devx + devy * devy;
        if devsq < 0.333 {
            self.draw_line(p0, p2);
            return;
        }
        let tol = 3.0;
        let n = 1 + fsqrt32(fsqrt32(tol * (devx * devx + devy * devy))) as usize;
        //println!("n = {}", n);
        let mut p = *p0;
        let nrecip = recip(n as f32);
        let mut t = 0.0;
        for _i in 0..n - 1 {
            t += nrecip;
            let pn = Point::lerp(t, &Point::lerp(t, p0, p1), &Point::lerp(t, p1, p2));
            self.draw_line(&p, &pn);
            p = pn;
        }
        self.draw_line(&p, p2);
    }

    /*
    fn get_bitmap_fancy(&self) -> Vec<u8> {
        let mut acc = 0.0;
        // This would translate really well to SIMD
        self.a[0..self.w * self.h].iter().map(|&a| {
            acc += a;
            (255.0 * acc.abs().min(1.0)) as u8
            //(255.5 * (0.5 + 0.4 * acc)) as u8
        }).collect()
    }
*/

    pub fn get_bitmap(&self) -> Vec<u8> {
        accumulate(&self.a[0..self.w * self.h])
    }
}
