//! Breast physics, ported from `com.wildfire.client.physics.BreastPhysics`
//! and `BothBreastsPhysics`.
//!
//! The original runs one spring per breast on every client tick. Horizon has
//! no Java mixins, so the port keeps the same numbers and feeds them the
//! movement, pose and swing data of a rendered `Mob`. Vehicle-specific
//! impulses (boats, minecarts, horses, pigs, striders) are not ported yet:
//! everything else follows the original formulas.

use crate::config::AvatarConfig;

const PI: f32 = std::f32::consts::PI;
/// `BreastPhysics.TIGHTNESS_REDUCTION_FACTOR`.
pub const TIGHTNESS_REDUCTION_FACTOR: f32 = 0.15;

/// The poses the physics reacts to (`LivingEntity.Pose`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPose {
    #[default]
    Standing,
    Crouching,
    Sleeping,
    Other,
}

/// One tick of a player's movement, as `BreastPhysics.update` reads it.
#[derive(Clone, Copy, Debug)]
pub struct EntityMotion {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    /// `LivingEntity.yBodyRot` and its previous tick, in degrees.
    pub body_yaw: f32,
    pub previous_body_yaw: f32,
    /// `WalkAnimationState.position` and `.speed`.
    pub limb_position: f32,
    pub limb_speed: f32,
    pub pose: MotionPose,
    pub swinging: bool,
    /// `currentSwing.hand().asArm(mainArm)`: true for the right arm.
    pub swing_right_arm: bool,
    pub age: u32,
}

/// The chest armour's effect on physics (`IGenderArmor`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Armor {
    pub covers: bool,
    pub hides: bool,
    pub resistance: f32,
    pub tightness: f32,
}

impl Armor {
    pub const NONE: Self = Self {
        covers: false,
        hides: false,
        resistance: 0.0,
        tightness: 0.0,
    };

    /// `WildfireClientHelper.getArmorConfig`: the data pack values of the
    /// original mod's `armor_data` files, by the chest item's name. Unknown
    /// chestplates get `IGenderArmor.DEFAULT` (resistance 0.5); elytra do not
    /// cover the breasts.
    pub fn from_chest(item: Option<&str>) -> Self {
        let Some(item) = item else {
            return Self::NONE;
        };
        let name = item.rsplit(':').next().unwrap_or(item).to_ascii_lowercase();
        if name.contains("elytra") {
            return Self {
                covers: false,
                hides: false,
                resistance: 0.0,
                tightness: 0.0,
            };
        }
        if name.contains("leather") {
            return Self {
                covers: true,
                hides: false,
                resistance: 0.3,
                tightness: 0.5,
            };
        }
        if name.contains("chainmail") || name.contains("chain") {
            return Self {
                covers: true,
                hides: false,
                resistance: 0.5,
                tightness: 0.2,
            };
        }
        if name.contains("gold") {
            return Self {
                covers: true,
                hides: false,
                resistance: 0.85,
                tightness: 0.0,
            };
        }
        if name.contains("iron")
            || name.contains("diamond")
            || name.contains("netherite")
            || name.contains("copper")
        {
            return Self {
                covers: true,
                hides: false,
                resistance: 1.0,
                tightness: 0.0,
            };
        }
        Self {
            covers: true,
            hides: false,
            resistance: 0.5,
            tightness: 0.0,
        }
    }
}

/// A tiny deterministic generator: the original uses a thread-local random.
/// Each breast keeps its own stream, so the two sides drift apart as they do
/// there, reproducibly across frames.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn next_bool(&mut self) -> bool {
        self.next_u32() & 1 != 0
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    fn between(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}

/// The values the renderer interpolates with the partial tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BreastValues {
    pub position_x: f32,
    pub position_y: f32,
    pub bounce_rotation: f32,
    pub breast_size: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BothValues {
    pub left: BreastValues,
    pub right: BreastValues,
}

#[derive(Clone, Debug)]
pub struct BreastPhysics {
    bounce_vel_x: f32,
    target_bounce_x: f32,
    velocity_x: f32,
    position_x: f32,
    pre_position_x: f32,

    bounce_vel: f32,
    target_bounce_y: f32,
    velocity: f32,
    position_y: f32,
    pre_position_y: f32,

    bounce_rot_vel: f32,
    target_rot_vel: f32,
    rot_velocity: f32,
    bounce_rotation: f32,
    pre_bounce_rotation: f32,

    breast_size: f32,
    pre_breast_size: f32,

    last_pose: Option<MotionPose>,
    last_swing_tick: i32,
    last_swing_duration: i32,
    pre_pos: Option<[f64; 3]>,
    random_b: i32,
    last_vertical_move_velocity: f64,
    rng: Rng,
}

impl BreastPhysics {
    pub fn new(seed: u64) -> Self {
        Self {
            bounce_vel_x: 0.0,
            target_bounce_x: 0.0,
            velocity_x: 0.0,
            position_x: 0.0,
            pre_position_x: 0.0,
            bounce_vel: 0.0,
            target_bounce_y: 0.0,
            velocity: 0.0,
            position_y: 0.0,
            pre_position_y: 0.0,
            bounce_rot_vel: 0.0,
            target_rot_vel: 0.0,
            rot_velocity: 0.0,
            bounce_rotation: 0.0,
            pre_bounce_rotation: 0.0,
            breast_size: 0.0,
            pre_breast_size: 0.0,
            last_pose: None,
            last_swing_tick: 0,
            last_swing_duration: 6,
            pre_pos: None,
            random_b: 1,
            last_vertical_move_velocity: 0.0,
            rng: Rng::new(seed),
        }
    }

    /// `BreastPhysics.update`, minus the vehicle cases.
    pub fn update(&mut self, m: &EntityMotion, cfg: &AvatarConfig, armor: Armor, override_armor: bool) {
        self.pre_position_y = self.position_y;
        self.pre_position_x = self.position_x;
        self.pre_bounce_rotation = self.bounce_rotation;
        self.pre_breast_size = self.breast_size;
        if self.pre_pos.is_none() {
            self.pre_pos = Some(m.position);
            return;
        }

        let mut target_breast_size = cfg.breasts.bust_size;
        let breast_weight = target_breast_size * 1.25;
        if !cfg.gender.can_have_breasts() {
            target_breast_size = 0.0;
        } else {
            let mut tightness = armor.tightness.clamp(0.0, 1.0);
            if override_armor {
                tightness = 0.0;
            }
            target_breast_size *= 1.0 - TIGHTNESS_REDUCTION_FACTOR * tightness;
        }
        if self.breast_size < target_breast_size {
            self.breast_size += (self.breast_size - target_breast_size).abs() / 2.0;
        } else {
            self.breast_size -= (self.breast_size - target_breast_size).abs() / 2.0;
        }

        let previous = self.pre_pos.unwrap_or(m.position);
        let motion = [
            m.position[0] - previous[0],
            m.position[1] - previous[1],
            m.position[2] - previous[2],
        ];
        self.pre_pos = Some(m.position);

        let mut bounce_intensity = target_breast_size
            * 3.0
            * ((cfg.breasts.bounce_multiplier * 3.0 * 100.0).round()) / 100.0;
        let mut resistance = armor.resistance.clamp(0.0, 1.0);
        if override_armor {
            resistance = 0.0;
        }
        bounce_intensity *= 1.0 - resistance;
        if !cfg.breasts.uniboob {
            bounce_intensity *= self.rng.between(0.5, 2.5);
        }

        self.tick_movement(m, motion, bounce_intensity, breast_weight);
        self.tick_pose(m, bounce_intensity);
        self.tick_arm_swing(m, bounce_intensity);
        self.finish_tick(cfg.breasts.floppiness);
    }

    fn calc_rotation(&self, m: &EntityMotion, bounce_intensity: f32) -> f32 {
        -((m.body_yaw - m.previous_body_yaw) / 15.0) * bounce_intensity
    }

    fn tick_movement(
        &mut self,
        m: &EntityMotion,
        motion: [f64; 3],
        bounce_intensity: f32,
        breast_weight: f32,
    ) {
        let vertical = m.velocity[1];
        if (self.last_vertical_move_velocity <= 0.0 && vertical > 0.0)
            || (self.last_vertical_move_velocity < 0.0 && vertical == 0.0)
        {
            self.random_b = if self.rng.next_bool() { -1 } else { 1 };
        }
        self.last_vertical_move_velocity = vertical;

        self.target_bounce_y = motion[1] as f32 * bounce_intensity;
        self.target_bounce_y += breast_weight;
        self.target_rot_vel = self.calc_rotation(m, bounce_intensity);
        self.target_rot_vel += motion[1] as f32 * bounce_intensity * self.random_b as f32;
        self.target_bounce_x = -self.calc_rotation(m, bounce_intensity) / 10.0;

        let speed_sq = m.velocity[0] * m.velocity[0]
            + m.velocity[1] * m.velocity[1]
            + m.velocity[2] * m.velocity[2];
        let mut speed_value = speed_sq as f32 / 0.2;
        speed_value = speed_value * speed_value * speed_value;
        if speed_value < 1.0 {
            speed_value = 1.0;
        }
        self.target_bounce_y += (m.limb_position * 0.6662 + PI).cos() * 0.5 * m.limb_speed * 0.5
            / speed_value;
    }

    fn tick_pose(&mut self, m: &EntityMotion, bounce_intensity: f32) {
        if self.last_pose != Some(m.pose) {
            match (m.pose, self.last_pose) {
                (MotionPose::Crouching, _) | (_, Some(MotionPose::Crouching)) => {
                    self.target_bounce_y += bounce_intensity;
                }
                (MotionPose::Sleeping, _) | (_, Some(MotionPose::Sleeping)) => {
                    self.target_bounce_y = bounce_intensity;
                }
                _ => {}
            }
            self.last_pose = Some(m.pose);
        }
    }

    /// `tickArmSwing`, reduced to the impulses that matter on screen. Horizon
    /// exposes the swinging arm and the attack animation rather than vanilla's
    /// `SwingState` duration; a six-tick swing is assumed, as the original's
    /// default.
    fn tick_arm_swing(&mut self, m: &EntityMotion, bounce_intensity: f32) {
        if !m.swinging {
            self.last_swing_tick = 0;
            self.last_swing_duration = 6;
            return;
        }
        if m.pose == MotionPose::Sleeping {
            return;
        }
        let duration = 6;
        let tick = (m.age % duration as u32) as i32;
        let is_swinging = true;
        let arm_right = m.swing_right_arm;
        if (duration > 1 || self.last_swing_duration > 1) && is_swinging {
            let amplifier = 1.0;
            let swing_tick_delta = tick - self.last_swing_tick;
            let progress = distance_from_median(0, self.last_swing_duration, tick);
            let swinging_toward_right = if progress > -0.2 { !arm_right } else { arm_right };
            let every_nth_tick = (duration - 1).clamp(1, 5);
            if m.age as i32 % every_nth_tick == 0 {
                self.target_bounce_y += (if self.rng.next_bool() { -0.25 } else { 0.25 })
                    * amplifier
                    * bounce_intensity;
                self.target_bounce_x =
                    0.325 * 1.0 * bounce_intensity * if arm_right { -1.0 } else { 1.0 };
            }
            if swing_tick_delta < 0 && self.last_swing_tick != self.last_swing_duration - 1 {
                self.target_rot_vel += (if arm_right { -4.0 } else { 4.0 })
                    * progress.abs()
                    * bounce_intensity;
            } else if is_swinging && duration > 1 {
                self.target_rot_vel += (if swinging_toward_right { -0.2 } else { 0.2 })
                    * amplifier
                    * bounce_intensity;
            }
            self.last_swing_tick = tick;
        }
        self.last_swing_duration = duration.max(1);
    }

    /// `finishTick`.
    fn finish_tick(&mut self, floppiness: f32) {
        let percent = floppiness;
        let bounce_amount = (0.45 * (1.0 - percent) + 0.15).clamp(0.15, 0.6);
        let delta = 2.25 - bounce_amount;
        let distance_from_min = (self.bounce_vel + 1.5).abs() * 0.5;
        let distance_from_max = (self.bounce_vel - 2.65).abs() * 0.5;
        if self.bounce_vel < -0.5 {
            self.target_bounce_y += distance_from_min;
        }
        if self.bounce_vel > 2.5 {
            self.target_bounce_y -= distance_from_max;
        }
        self.target_bounce_y = self.target_bounce_y.clamp(-1.5, 2.5);
        self.target_rot_vel = self.target_rot_vel.clamp(-25.0, 25.0);

        self.velocity = lerp(
            bounce_amount,
            self.velocity,
            (self.target_bounce_y - self.bounce_vel) * delta,
        );
        self.bounce_vel += self.velocity * percent * 1.1625;
        self.velocity_x = lerp(
            bounce_amount,
            self.velocity_x,
            (self.target_bounce_x - self.bounce_vel_x) * delta,
        );
        self.bounce_vel_x += self.velocity_x * percent;
        self.rot_velocity = lerp(
            bounce_amount,
            self.rot_velocity,
            (self.target_rot_vel - self.bounce_rot_vel) * delta,
        );
        self.bounce_rot_vel += self.rot_velocity * percent;
        self.bounce_rotation = self.bounce_rot_vel;
        self.position_x = self.bounce_vel_x;
        self.position_y = self.bounce_vel;
        if self.position_y < -0.5 {
            self.position_y = -0.5;
        }
        if self.position_y > 1.5 {
            self.position_y = 1.5;
            self.velocity = 0.0;
        }
    }

    pub fn values(&self, partial: f32) -> BreastValues {
        let partial = partial.clamp(0.0, 1.0);
        BreastValues {
            position_x: lerp(partial, self.pre_position_x, self.position_x),
            position_y: lerp(partial, self.pre_position_y, self.position_y),
            bounce_rotation: lerp(partial, self.pre_bounce_rotation, self.bounce_rotation),
            breast_size: lerp(partial, self.pre_breast_size, self.breast_size),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BothBreasts {
    pub left: BreastPhysics,
    pub right: BreastPhysics,
}

impl BothBreasts {
    pub fn new(entity_id: u32) -> Self {
        Self {
            left: BreastPhysics::new(0x9E37_79B9_7F4A_7C15 ^ u64::from(entity_id)),
            right: BreastPhysics::new(0xD1B5_4A32_D192_ED03 ^ u64::from(entity_id).rotate_left(17)),
        }
    }

    pub fn update(&mut self, m: &EntityMotion, cfg: &AvatarConfig, armor: Armor, override_armor: bool) {
        self.left.update(m, cfg, armor, override_armor);
        self.right.update(m, cfg, armor, override_armor);
    }

    pub fn values(&self, partial: f32) -> BothValues {
        BothValues {
            left: self.left.values(partial),
            right: self.right.values(partial),
        }
    }
}

fn lerp(amount: f32, from: f32, to: f32) -> f32 {
    from + amount * (to - from)
}

/// `BreastPhysics.distanceFromMedian`.
fn distance_from_median(p1: i32, p2: i32, point: i32) -> f32 {
    if p1 >= p2 {
        return 0.0;
    }
    let mut point = point as f32;
    if point <= p1 as f32 || point >= p2 as f32 {
        return 0.0;
    }
    let median = (p2 - p1) as f32 / 2.0;
    point -= p1 as f32;
    if point > median {
        point = -(median - (point - median));
    }
    point / median
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Gender;

    fn motion() -> EntityMotion {
        EntityMotion {
            position: [0.0, 64.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            body_yaw: 0.0,
            previous_body_yaw: 0.0,
            limb_position: 0.0,
            limb_speed: 0.0,
            pose: MotionPose::Standing,
            swinging: false,
            swing_right_arm: false,
            age: 0,
        }
    }

    #[test]
    fn a_male_has_no_breast_size() {
        let mut physics = BreastPhysics::new(1);
        let mut m = motion();
        physics.update(&m, &AvatarConfig::default(), Armor::NONE, false);
        for age in 1..40 {
            m.age = age;
            physics.update(&m, &AvatarConfig::default(), Armor::NONE, false);
        }
        assert!(physics.values(1.0).breast_size < 0.001);
    }

    #[test]
    fn a_female_reaches_the_target_size() {
        let mut physics = BreastPhysics::new(1);
        let mut m = motion();
        let mut cfg = AvatarConfig::default();
        cfg.gender = Gender::Female;
        cfg.breasts.bust_size = 0.6;
        physics.update(&m, &cfg, Armor::NONE, false);
        for age in 1..80 {
            m.age = age;
            physics.update(&m, &cfg, Armor::NONE, false);
        }
        assert!((physics.values(1.0).breast_size - 0.6).abs() < 0.01);
    }

    #[test]
    fn armor_resistance_damps_bounce() {
        let mut cfg = AvatarConfig::default();
        cfg.gender = Gender::Female;
        let armor = Armor::from_chest(Some("minecraft:iron_chestplate"));
        assert_eq!(armor.resistance, 1.0);
        let mut physics = BreastPhysics::new(7);
        let mut m = motion();
        m.velocity[1] = 0.5;
        for age in 0..20 {
            m.age = age;
            m.previous_body_yaw = 0.0;
            m.body_yaw = 10.0;
            physics.update(&m, &cfg, armor, false);
            m.body_yaw = 0.0;
        }
        assert!(physics.values(1.0).bounce_rotation.abs() < 0.001);
    }
}
