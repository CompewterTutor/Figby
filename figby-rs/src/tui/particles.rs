use serde::Deserialize;

use super::layers::BlendMode;

#[derive(Debug, Clone, Deserialize)]
pub struct ParticleConfig {
    #[serde(default)]
    pub emitter_x: f64,
    #[serde(default)]
    pub emitter_y: f64,
    #[serde(default = "default_spawn_rate")]
    pub spawn_rate: f64,
    #[serde(default = "default_lifetime")]
    pub lifetime_min: f64,
    #[serde(default = "default_lifetime")]
    pub lifetime_max: f64,
    #[serde(default)]
    pub velocity_x_min: f64,
    #[serde(default)]
    pub velocity_x_max: f64,
    #[serde(default)]
    pub velocity_y_min: f64,
    #[serde(default)]
    pub velocity_y_max: f64,
    #[serde(default)]
    pub acceleration_x: f64,
    #[serde(default)]
    pub acceleration_y: f64,
    #[serde(default)]
    pub size: u8,
    #[serde(default)]
    pub color_r: Option<u8>,
    #[serde(default)]
    pub color_g: Option<u8>,
    #[serde(default)]
    pub color_b: Option<u8>,
    #[serde(default = "default_char")]
    pub character: char,
    #[serde(default = "default_opacity")]
    pub opacity: u8,
    #[serde(default)]
    pub blend_mode: Option<String>,
}

fn default_spawn_rate() -> f64 {
    10.0
}

fn default_lifetime() -> f64 {
    1.0
}

fn default_char() -> char {
    '*'
}

fn default_opacity() -> u8 {
    255
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            emitter_x: 0.0,
            emitter_y: 0.0,
            spawn_rate: default_spawn_rate(),
            lifetime_min: default_lifetime(),
            lifetime_max: default_lifetime(),
            velocity_x_min: 0.0,
            velocity_x_max: 0.0,
            velocity_y_min: 0.0,
            velocity_y_max: 0.0,
            acceleration_x: 0.0,
            acceleration_y: 0.0,
            size: 1,
            color_r: None,
            color_g: None,
            color_b: None,
            character: default_char(),
            opacity: default_opacity(),
            blend_mode: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub remaining_lifetime: f64,
    pub size: u8,
    pub color: Option<(u8, u8, u8)>,
    pub character: char,
    pub opacity: u8,
    pub blend_mode: BlendMode,
}

impl Particle {
    fn new(config: &ParticleConfig) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let lifetime = if config.lifetime_min < config.lifetime_max {
            rng.gen_range(config.lifetime_min..config.lifetime_max)
        } else {
            config.lifetime_min
        };

        let vx = if config.velocity_x_min < config.velocity_x_max {
            rng.gen_range(config.velocity_x_min..config.velocity_x_max)
        } else {
            config.velocity_x_min
        };

        let vy = if config.velocity_y_min < config.velocity_y_max {
            rng.gen_range(config.velocity_y_min..config.velocity_y_max)
        } else {
            config.velocity_y_min
        };

        let color = match (config.color_r, config.color_g, config.color_b) {
            (Some(r), Some(g), Some(b)) => Some((r, g, b)),
            _ => None,
        };

        let blend_mode = config
            .blend_mode
            .as_deref()
            .and_then(|s| s.parse::<BlendMode>().ok())
            .unwrap_or(BlendMode::Normal);

        Self {
            x: config.emitter_x,
            y: config.emitter_y,
            vx,
            vy,
            remaining_lifetime: lifetime,
            size: config.size,
            color,
            character: config.character,
            opacity: config.opacity,
            blend_mode,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParticleSystem {
    pub config: ParticleConfig,
    pub active_particles: Vec<Particle>,
    pub age: f64,
    spawn_rate_accumulator: f64,
    paused: bool,
}

impl ParticleSystem {
    pub fn new(config: ParticleConfig) -> Self {
        Self {
            config,
            active_particles: Vec::new(),
            age: 0.0,
            spawn_rate_accumulator: 0.0,
            paused: false,
        }
    }

    pub fn update(&mut self, dt: f64) {
        if self.paused || dt <= 0.0 {
            return;
        }

        self.age += dt;

        // Spawn new particles
        self.spawn_rate_accumulator += self.config.spawn_rate * dt;
        let to_spawn = self.spawn_rate_accumulator.floor() as usize;
        self.spawn_rate_accumulator -= to_spawn as f64;

        for _ in 0..to_spawn {
            self.active_particles.push(Particle::new(&self.config));
        }

        // Update existing particles
        let ax = self.config.acceleration_x;
        let ay = self.config.acceleration_y;

        for p in &mut self.active_particles {
            p.vx += ax * dt;
            p.vy += ay * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.remaining_lifetime -= dt;
        }

        // Remove expired particles
        self.active_particles.retain(|p| p.remaining_lifetime > 0.0);
    }

    pub fn active_count(&self) -> usize {
        self.active_particles.len()
    }

    pub fn clear(&mut self) {
        self.active_particles.clear();
        self.spawn_rate_accumulator = 0.0;
        self.age = 0.0;
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ParticleConfig {
        ParticleConfig {
            spawn_rate: 10.0,
            lifetime_min: 2.0,
            lifetime_max: 2.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_particle_spawn() {
        let mut system = ParticleSystem::new(test_config());
        system.update(1.0);
        assert!(system.active_count() >= 1);
        for p in &system.active_particles {
            assert_eq!(p.x, 0.0);
            assert_eq!(p.y, 0.0);
        }
    }

    #[test]
    fn test_particle_update_motion() {
        let config = ParticleConfig {
            spawn_rate: 10.0,
            lifetime_min: 2.0,
            lifetime_max: 2.0,
            velocity_x_min: 5.0,
            velocity_x_max: 5.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(0.1);
        assert!(system.active_count() >= 1);
        let p = &system.active_particles[0];
        assert!((p.x - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_particle_expire() {
        let config = ParticleConfig {
            spawn_rate: 10.0,
            lifetime_min: 0.5,
            lifetime_max: 0.5,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(0.3);
        assert!(system.active_count() > 0);
        system.update(0.6);
        assert_eq!(system.active_count(), 0);
    }

    #[test]
    fn test_particle_acceleration() {
        let config = ParticleConfig {
            spawn_rate: 1.0,
            lifetime_min: 2.0,
            lifetime_max: 2.0,
            acceleration_y: 5.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(1.0);
        assert_eq!(system.active_count(), 1);
        let p = &system.active_particles[0];
        assert!((p.vy - 5.0).abs() < 0.01);
        assert!((p.y - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_particle_spawn_rate_accumulator() {
        let config = ParticleConfig {
            spawn_rate: 0.5,
            lifetime_min: 10.0,
            lifetime_max: 10.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(1.0);
        let after_1s = system.active_count();
        system.update(3.0);
        assert_eq!(after_1s + system.active_count(), 2);
    }

    #[test]
    fn test_particle_full_lifecycle() {
        let config = ParticleConfig {
            spawn_rate: 1.0,
            lifetime_min: 3.0,
            lifetime_max: 3.0,
            velocity_x_min: 2.0,
            velocity_x_max: 2.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(1.0);
        assert_eq!(system.active_count(), 1);
        let p = &system.active_particles[0];
        assert!((p.x - 2.0).abs() < 0.01);

        system.update(1.0);
        assert_eq!(system.active_count(), 2);
        let p1 = &system.active_particles[0];
        let p2 = &system.active_particles[1];
        assert!((p1.x - 4.0).abs() < 0.01);
        assert!((p2.x - 2.0).abs() < 0.01);

        // Both still alive at t=2.99
        system.update(0.99);
        assert_eq!(system.active_count(), 2);

        // Expire after lifetime exceeded for oldest (P2 still alive, P3 spawned)
        system.update(1.01);
        assert_eq!(system.active_count(), 2);
    }

    #[test]
    fn test_particle_color_from_config() {
        let config = ParticleConfig {
            spawn_rate: 10.0,
            lifetime_min: 2.0,
            lifetime_max: 2.0,
            color_r: Some(255),
            color_g: Some(128),
            color_b: Some(64),
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(0.1);
        let p = &system.active_particles[0];
        assert_eq!(p.color, Some((255, 128, 64)));
    }

    #[test]
    fn test_particle_no_color_default() {
        let config = ParticleConfig {
            spawn_rate: 10.0,
            lifetime_min: 2.0,
            lifetime_max: 2.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(0.1);
        let p = &system.active_particles[0];
        assert_eq!(p.color, None);
    }

    #[test]
    fn test_particle_system_pause_resume() {
        let config = ParticleConfig {
            spawn_rate: 100.0,
            lifetime_min: 10.0,
            lifetime_max: 10.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(1.0);
        let count_before_pause = system.active_count();
        assert!(count_before_pause > 0);

        system.pause();
        assert!(system.is_paused());
        system.update(1.0);
        assert_eq!(system.active_count(), count_before_pause);

        system.resume();
        assert!(!system.is_paused());
        system.update(1.0);
        assert!(system.active_count() > count_before_pause);
    }

    #[test]
    fn test_particle_system_clear() {
        let config = ParticleConfig {
            spawn_rate: 100.0,
            lifetime_min: 10.0,
            lifetime_max: 10.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config);
        system.update(1.0);
        assert!(system.active_count() > 0);
        system.clear();
        assert_eq!(system.active_count(), 0);
        assert_eq!(system.age, 0.0);
    }

    #[test]
    fn test_particle_negative_dt_noop() {
        let mut system = ParticleSystem::new(test_config());
        system.update(1.0);
        let count = system.active_count();
        system.update(-1.0);
        assert_eq!(system.active_count(), count);
    }

    #[test]
    fn test_particle_zero_dt_noop() {
        let mut system = ParticleSystem::new(test_config());
        system.update(1.0);
        let count = system.active_count();
        system.update(0.0);
        assert_eq!(system.active_count(), count);
    }
}
