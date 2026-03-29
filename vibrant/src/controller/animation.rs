use std::f32::consts::PI;

// ----------------------------------
//       STRUCTS DECLARATIONS
// ----------------------------------

#[derive(Debug, Clone)]
pub struct Keyframe {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub duration: f32, // seconds to reach this keyframe from the previous one
}

#[derive(Debug)]
pub enum Animation {
    Sequence(AnimationSequence),
    Orbit(OrbitAnimation),
    Cinematic(CinematicAnimation),
    SpiralZoom(SpiralZoomAnimation),
    FigureEight(FigureEightAnimation),
}

#[derive(Debug, Clone)]
pub struct AnimationSequence {
    pub keyframes: Vec<Keyframe>,
    pub current: usize,
    pub elapsed: f32,
    pub playing: bool,
    pub finished: bool,
}

#[derive(Debug)]
pub struct OrbitAnimation {
    pub pitch: f32,
    pub distance: f32,
    pub speed: f32,    // radians per second, 2*PI = one full rotation
    pub duration: f32, // total seconds before stopping
    pub elapsed: f32,
    pub playing: bool,
    pub finished: bool,
}

#[derive(Debug)]
pub struct CinematicAnimation {
    pub start_distance: f32,
    pub elapsed: f32,
    pub duration: f32,
    pub playing: bool,
    pub finished: bool,
}

#[derive(Debug)]
pub struct SpiralZoomAnimation {
    pub start_distance: f32,
    pub elapsed: f32,
    pub duration: f32,
    pub playing: bool,
    pub finished: bool,
}

#[derive(Debug)]
pub struct FigureEightAnimation {
    pub start_distance: f32,
    pub elapsed: f32,
    pub duration: f32,
    pub playing: bool,
    pub finished: bool,
}

// ----------------------------------
//      STRUCTS IMPLEMENTATIONS
// ----------------------------------

impl AnimationSequence {
    pub fn new(keyframes: Vec<Keyframe>) -> Self {
        Self {
            keyframes,
            current: 0,
            elapsed: 0.0,
            playing: false,
            finished: false,
        }
    }

    pub fn start(&mut self) {
        self.current = 0;
        self.elapsed = 0.0;
        self.playing = true;
        self.finished = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    // Returns (yaw, pitch, distance) interpolated for the current time
    pub fn update(&mut self, dt: f32) -> Option<(f32, f32, f32)> {
        if !self.playing || self.finished {
            return None;
        }

        if self.keyframes.len() < 2 {
            return None;
        }

        self.elapsed += dt;

        let from = &self.keyframes[self.current];
        let to = &self.keyframes[self.current + 1];

        let t = (self.elapsed / from.duration).clamp(0.0, 1.0);
        let t_ease = ease_in_out(t);

        let yaw = lerp(from.yaw, to.yaw, t_ease);
        let pitch = lerp(from.pitch, to.pitch, t_ease);
        let distance = lerp(from.distance, to.distance, t_ease);

        // Advance to next keyframe if current is done
        if self.elapsed >= from.duration {
            self.elapsed -= from.duration;
            self.current += 1;

            // Check if we've reached the last keyframe
            if self.current >= self.keyframes.len() - 1 {
                self.playing = false;
                self.finished = true;
            }
        }

        Some((yaw, pitch, distance))
    }
}

impl Animation {
    pub fn update(&mut self, dt: f32) -> Option<(f32, f32, f32)> {
        match self {
            Animation::Sequence(s) => s.update(dt),
            Animation::Orbit(o) => o.update(dt),
            Animation::Cinematic(c) => c.update(dt),
            Animation::SpiralZoom(s) => s.update(dt),
            Animation::FigureEight(f) => f.update(dt),
        }
    }

    pub fn finished(&self) -> bool {
        match self {
            Animation::Sequence(s) => s.finished,
            Animation::Orbit(o) => o.finished,
            Animation::Cinematic(c) => c.finished,
            Animation::SpiralZoom(s) => s.finished,
            Animation::FigureEight(f) => f.finished,
        }
    }
}

impl OrbitAnimation {
    pub fn new(pitch: f32, distance: f32, speed: f32, duration: f32) -> Self {
        Self {
            pitch,
            distance,
            speed,
            duration,
            elapsed: 0.0,
            playing: false,
            finished: false,
        }
    }

    pub fn start(&mut self) {
        self.elapsed = 0.0;
        self.playing = true;
        self.finished = false;
    }

    pub fn update(&mut self, dt: f32) -> Option<(f32, f32, f32)> {
        if !self.playing || self.finished {
            return None;
        }

        self.elapsed += dt;

        let yaw = self.elapsed * self.speed;
        let pitch = self.pitch;
        let distance = self.distance;

        if self.elapsed >= self.duration {
            self.playing = false;
            self.finished = true;
        }

        Some((yaw, pitch, distance))
    }
}

impl CinematicAnimation {
    pub fn new(start_distance: f32) -> Self {
        Self {
            start_distance,
            elapsed: 0.0,
            duration: 20.0, // total duration in seconds
            playing: false,
            finished: false,
        }
    }

    pub fn start(&mut self) {
        self.elapsed = 0.0;
        self.playing = true;
        self.finished = false;
    }

    pub fn update(&mut self, dt: f32) -> Option<(f32, f32, f32)> {
        if !self.playing || self.finished {
            return None;
        }

        self.elapsed += dt;
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0); // 0.0 -> 1.0 over full duration

        // Yaw: one full rotation over the whole duration
        let yaw = t * 2.0 * PI;

        // Pitch: starts slightly low, rises over the top, comes back down — one smooth wave
        let pitch = 0.15 + 0.35 * (t * PI).sin();

        // Distance: starts far, pushes in close, pulls back out — smooth zoom in/out
        let base = self.start_distance;
        let distance = base + (base * 0.5) * (1.0 - (t * 2.0 * PI).cos()) / 2.0
            - (base * 0.3) * (t * PI).sin();

        if self.elapsed >= self.duration {
            self.playing = false;
            self.finished = true;
        }

        Some((yaw, pitch, distance.clamp(0.1, 5.0)))
    }
}

impl SpiralZoomAnimation {
    pub fn new(start_distance: f32) -> Self {
        Self {
            start_distance,
            elapsed: 0.0,
            duration: 12.0,
            playing: false,
            finished: false,
        }
    }

    pub fn start(&mut self) {
        self.elapsed = 0.0;
        self.playing = true;
        self.finished = false;
    }

    pub fn update(&mut self, dt: f32) -> Option<(f32, f32, f32)> {
        if !self.playing || self.finished {
            return None;
        }

        self.elapsed += dt;
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);

        // 3/4 rotation during the whole animation
        let yaw = t * PI * 1.5;

        // Pitch slowly rises during spiral
        let pitch = 0.1 + t * 0.3;

        // Zoom in for first half, back out for second half — smooth using sine
        let zoom_t = (t * PI).sin(); // 0 → 1 → 0 over full duration
        let distance = self.start_distance * (1.0 - zoom_t * 0.65);

        if self.elapsed >= self.duration {
            self.playing = false;
            self.finished = true;
        }

        Some((yaw, pitch, distance.clamp(0.1, 5.0)))
    }
}

impl FigureEightAnimation {
    pub fn new(start_distance: f32) -> Self {
        Self {
            start_distance,
            elapsed: 0.0,
            duration: 16.0,
            playing: false,
            finished: false,
        }
    }

    pub fn start(&mut self) {
        self.elapsed = 0.0;
        self.playing = true;
        self.finished = false;
    }

    pub fn update(&mut self, dt: f32) -> Option<(f32, f32, f32)> {
        if !self.playing || self.finished {
            return None;
        }

        self.elapsed += dt;
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);

        // Continuous full rotation
        let yaw = t * 2.0 * PI;

        // Pitch oscillates twice per rotation — figure-8 shape
        let pitch = (t * 2.0 * PI * 2.0).sin() * 0.45;

        // Subtle distance pulse — breathes in and out twice
        let distance =
            self.start_distance + (t * 2.0 * PI * 2.0).cos() * self.start_distance * 0.15;

        if self.elapsed >= self.duration {
            self.playing = false;
            self.finished = true;
        }

        Some((yaw, pitch, distance.clamp(0.1, 5.0)))
    }
}

// ----------------------------------
//          HELPER METHODS
// ----------------------------------

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease_in_out(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

// ----------------------------------
//       PRE-DEFINED ANIMATIONS
// ----------------------------------

pub fn orbit_360(distance: f32) -> Animation {
    // Full slow orbit around the brain
    let mut orbit = OrbitAnimation::new(
        0.15,           // pitch — slight tilt down to see the top
        distance,       // distance
        2.0 * PI / 8.0, // speed — one full rotation in 8 seconds
        8.0,            // duration — 8 seconds total
    );
    orbit.start();
    Animation::Orbit(orbit)
}

pub fn axial_sweep(distance: f32) -> Animation {
    // Move through standard anatomical views
    let mut seq = AnimationSequence::new(vec![
        Keyframe {
            yaw: 0.0,
            pitch: 0.1,
            distance,
            duration: 0.01,
        },
        Keyframe {
            yaw: 0.3,
            pitch: 0.2,
            distance: 0.5,
            duration: 3.0,
        },
        Keyframe {
            yaw: 0.3,
            pitch: 0.2,
            distance,
            duration: 1.5,
        },
        Keyframe {
            yaw: PI / 2.0,
            pitch: 0.1,
            distance: 0.5,
            duration: 3.0,
        },
        Keyframe {
            yaw: PI / 2.0,
            pitch: 0.1,
            distance,
            duration: 1.5,
        },
        Keyframe {
            yaw: PI,
            pitch: 0.2,
            distance: 0.5,
            duration: 3.0,
        },
        Keyframe {
            yaw: PI,
            pitch: 0.2,
            distance,
            duration: 1.5,
        },
        Keyframe {
            yaw: PI / 4.0,
            pitch: 0.3,
            distance,
            duration: 2.0,
        },
    ]);
    seq.start();
    Animation::Sequence(seq)
}

pub fn zoom_to_regions(distance: f32) -> Animation {
    // Orbit while zooming in and out to highlight regions
    let mut seq = AnimationSequence::new(vec![
        Keyframe {
            yaw: 0.0,
            pitch: 0.1,
            distance: distance,
            duration: 0.01,
        }, // start
        Keyframe {
            yaw: 0.3,
            pitch: 0.2,
            distance: 0.5,
            duration: 3.0,
        }, // zoom into front-right
        Keyframe {
            yaw: 0.3,
            pitch: 0.2,
            distance: distance,
            duration: 1.5,
        }, // zoom back out
        Keyframe {
            yaw: PI / 2.0,
            pitch: 0.1,
            distance: 0.5,
            duration: 3.0,
        }, // zoom into right side
        Keyframe {
            yaw: PI / 2.0,
            pitch: 0.1,
            distance: distance,
            duration: 1.5,
        }, // zoom back out
        Keyframe {
            yaw: PI,
            pitch: 0.2,
            distance: 0.5,
            duration: 3.0,
        }, // zoom into back
        Keyframe {
            yaw: PI,
            pitch: 0.2,
            distance: distance,
            duration: 1.5,
        }, // zoom back out
        Keyframe {
            yaw: PI / 4.0,
            pitch: 0.3,
            distance: distance,
            duration: 2.0,
        }, // settle on 3/4 view
    ]);
    seq.start();
    Animation::Sequence(seq)
}

pub fn cinematic(distance: f32) -> Animation {
    // Slow dramatic reveal — start from far, rotate in, settle
    let mut anim = CinematicAnimation::new(distance);
    anim.start();
    Animation::Cinematic(anim)
}

pub fn hemisphere_split(distance: f32) -> Animation {
    let mut seq = AnimationSequence::new(vec![
        Keyframe {
            yaw: 0.0,
            pitch: 0.1,
            distance,
            duration: 0.01,
        }, // start front
        Keyframe {
            yaw: 0.0,
            pitch: 0.1,
            distance,
            duration: 2.0,
        }, // hold front
        Keyframe {
            yaw: -PI / 2.0,
            pitch: 0.1,
            distance,
            duration: 3.0,
        }, // rotate to left hemisphere
        Keyframe {
            yaw: -PI / 2.0,
            pitch: 0.1,
            distance,
            duration: 2.0,
        }, // hold left
        Keyframe {
            yaw: 0.0,
            pitch: 0.1,
            distance,
            duration: 2.0,
        }, // back to front
        Keyframe {
            yaw: PI / 2.0,
            pitch: 0.1,
            distance,
            duration: 3.0,
        }, // rotate to right hemisphere
        Keyframe {
            yaw: PI / 2.0,
            pitch: 0.1,
            distance,
            duration: 2.0,
        }, // hold right
        Keyframe {
            yaw: 0.0,
            pitch: 0.1,
            distance,
            duration: 2.0,
        }, // back to front
    ]);
    seq.start();
    Animation::Sequence(seq)
}

pub fn top_down_dive(distance: f32) -> Animation {
    let mut seq = AnimationSequence::new(vec![
        Keyframe {
            yaw: 0.0,
            pitch: PI / 2.0,
            distance,
            duration: 0.01,
        }, // start top
        Keyframe {
            yaw: 0.0,
            pitch: PI / 2.0,
            distance,
            duration: 2.0,
        }, // hold top
        Keyframe {
            yaw: 0.3,
            pitch: PI / 3.0,
            distance: distance * 0.85,
            duration: 3.0,
        }, // begin descent
        Keyframe {
            yaw: 0.6,
            pitch: PI / 5.0,
            distance: distance * 0.75,
            duration: 3.0,
        }, // mid descent
        Keyframe {
            yaw: 0.9,
            pitch: PI / 8.0,
            distance: distance * 0.7,
            duration: 3.0,
        }, // near eye level
        Keyframe {
            yaw: 1.2,
            pitch: 0.05,
            distance,
            duration: 3.0,
        }, // eye level
        Keyframe {
            yaw: PI / 4.0,
            pitch: PI / 6.0,
            distance,
            duration: 2.0,
        }, // settle 3/4
    ]);
    seq.start();
    Animation::Sequence(seq)
}

pub fn pendulum(distance: f32) -> Animation {
    // Rocks left and right while drifting forward
    let steps = 40;
    let duration = 12.0;
    let dt = duration / steps as f32;
    let mut keyframes = vec![Keyframe {
        yaw: -0.4,
        pitch: 0.15,
        distance,
        duration: 0.01,
    }];
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let yaw = (t * PI * 4.0).sin() * 0.4  // pendulum swing
            + t * PI * 0.5; // slow forward drift
        let pitch = 0.15 + (t * PI * 2.0).sin() * 0.08; // gentle pitch wave
        keyframes.push(Keyframe {
            yaw,
            pitch,
            distance,
            duration: dt,
        });
    }
    let mut seq = AnimationSequence::new(keyframes);
    seq.start();
    Animation::Sequence(seq)
}

pub fn spiral_zoom(distance: f32) -> Animation {
    let mut anim = SpiralZoomAnimation::new(distance);
    anim.start();
    Animation::SpiralZoom(anim)
}

pub fn figure_eight(distance: f32) -> Animation {
    let mut anim = FigureEightAnimation::new(distance);
    anim.start();
    Animation::FigureEight(anim)
}

pub fn slow_reveal(distance: f32) -> Animation {
    let mut seq = AnimationSequence::new(vec![
        Keyframe {
            yaw: PI,
            pitch: -0.1,
            distance: distance * 2.5,
            duration: 0.01,
        }, // start — far, facing away
        Keyframe {
            yaw: PI * 1.3,
            pitch: 0.0,
            distance: distance * 1.8,
            duration: 3.0,
        }, // begin turn
        Keyframe {
            yaw: PI * 1.6,
            pitch: 0.05,
            distance: distance * 1.3,
            duration: 3.0,
        }, // mid turn, closing in
        Keyframe {
            yaw: PI * 1.9,
            pitch: 0.1,
            distance: distance * 1.0,
            duration: 3.0,
        }, // nearly front
        Keyframe {
            yaw: PI * 2.0,
            pitch: 0.15,
            distance,
            duration: 2.0,
        }, // full front, settled
        Keyframe {
            yaw: PI * 2.0,
            pitch: 0.15,
            distance,
            duration: 3.0,
        }, // hold
    ]);
    seq.start();
    Animation::Sequence(seq)
}
