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
    TopDownDive(TopDownDiveAnimation),
    Pendulum(PendulumAnimation),
    SlowReveal(SlowRevealAnimation),
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

#[derive(Debug)]
pub struct TopDownDiveAnimation {
    pub start_distance: f32,
    pub elapsed: f32,
    pub duration: f32,
    pub playing: bool,
    pub finished: bool,
}

#[derive(Debug)]
pub struct PendulumAnimation {
    pub start_distance: f32,
    pub elapsed: f32,
    pub duration: f32,
    pub playing: bool,
    pub finished: bool,
}

#[derive(Debug)]
pub struct SlowRevealAnimation {
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
            Animation::TopDownDive(t) => t.update(dt),
            Animation::Pendulum(p) => p.update(dt),
            Animation::SlowReveal(s) => s.update(dt),
        }
    }

    pub fn finished(&self) -> bool {
        match self {
            Animation::Sequence(s) => s.finished,
            Animation::Orbit(o) => o.finished,
            Animation::Cinematic(c) => c.finished,
            Animation::SpiralZoom(s) => s.finished,
            Animation::FigureEight(f) => f.finished,
            Animation::TopDownDive(t) => t.finished,
            Animation::Pendulum(p) => p.finished,
            Animation::SlowReveal(s) => s.finished,
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
            start_distance: start_distance * 1.5,
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

        // Zoom in for first half, back out for second half
        let zoom_t = (t * PI).sin(); // 0 -> 1 -> 0 over full duration
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

impl TopDownDiveAnimation {
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

        // Yaw: slow drift forward during descent
        let yaw = t * 1.2;

        // Pitch: starts at top (PI/2), smoothly descends to eye level
        // Hold at top for first 15%, then descend
        let pitch = if t < 0.15 {
            PI / 2.0
        } else {
            let t2 = (t - 0.15) / 0.85;
            let t2_ease = ease_in_out(t2);
            lerp(PI / 2.0, PI / 6.0, t2_ease)
        };

        // Distance: zooms in slightly during descent
        let zoom = 1.0 - (t * PI).sin() * 0.3;
        let distance = self.start_distance * zoom;

        if self.elapsed >= self.duration {
            self.playing = false;
            self.finished = true;
        }

        Some((yaw, pitch, distance.clamp(0.1, 5.0)))
    }
}

impl PendulumAnimation {
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

        // Pendulum swing — 2 full swings left/right with decay
        let decay = 1.0 - t * 0.5; // amplitude reduces over time
        let yaw = (t * PI * 4.0).sin() * 0.4 * decay + t * PI * 0.5; // slow forward drift

        // Pitch: gentle wave
        let pitch = 0.15 + (t * PI * 2.0).sin() * 0.08;

        let distance = self.start_distance;

        if self.elapsed >= self.duration {
            self.playing = false;
            self.finished = true;
        }

        Some((yaw, pitch, distance))
    }
}

impl SlowRevealAnimation {
    pub fn new(start_distance: f32) -> Self {
        Self {
            start_distance,
            elapsed: 0.0,
            duration: 14.0,
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

        // Yaw: starts facing away (PI), rotates to front (2*PI) smoothly
        // Hold at end for last 20%
        let t_yaw = if t < 0.8 { ease_in_out(t / 0.8) } else { 1.0 };
        let yaw = lerp(PI, PI * 2.0, t_yaw);

        // Pitch: starts slightly negative, rises to slight positive
        let pitch = lerp(-0.1, 0.15, ease_in_out(t));

        // Distance: starts far (2.5x), closes in to normal distance
        let t_dist = ease_in_out((t * 1.2).min(1.0));
        let distance = lerp(self.start_distance * 2.5, self.start_distance, t_dist);

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
        0.15,            // pitch — slight tilt down to see the top
        distance * 1.25, // distance
        2.0 * PI / 8.0,  // speed — one full rotation in 8 seconds
        8.0,             // duration — 8 seconds total
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
    let mut anim = TopDownDiveAnimation::new(distance);
    anim.start();
    Animation::TopDownDive(anim)
}

pub fn pendulum(distance: f32) -> Animation {
    let mut anim = PendulumAnimation::new(distance);
    anim.start();
    Animation::Pendulum(anim)
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
    let mut anim = SlowRevealAnimation::new(distance);
    anim.start();
    Animation::SlowReveal(anim)
}
