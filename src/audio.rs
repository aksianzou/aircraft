use agb::fixnum::{Num, num};
use agb::include_wav;
use agb::sound::mixer::{ChannelId, SoundChannel, SoundData};

use crate::world::*;

/// Menu loop. The recorded melody, a quiet bass, and a soft third under it.
pub(crate) static MUSIC: SoundData = include_wav!("sfx/music.wav");
/// End roll. The recorded melody on its own, a little softer.
pub(crate) static END: SoundData = include_wav!("sfx/end.wav");
pub(crate) static GUN_SFX: SoundData = include_wav!("sfx/gun.wav");
pub(crate) static WHISTLE_SFX: SoundData = include_wav!("sfx/whistle.wav");
pub(crate) static BOOM_SFX: SoundData = include_wav!("sfx/boom.wav");
pub(crate) static FAR_SFX: SoundData = include_wav!("sfx/far.wav");
pub(crate) static RUMBLE_SFX: SoundData = include_wav!("sfx/rumble.wav");
pub(crate) static BIRD_SFX: SoundData = include_wav!("sfx/bird.wav");
pub(crate) static BIRD2_SFX: SoundData = include_wav!("sfx/bird2.wav");
pub(crate) static ENGINE_SFX: SoundData = include_wav!("sfx/engine.wav");
pub(crate) static VOICE_SFX: SoundData = include_wav!("sfx/voice.wav");
pub(crate) static MARK_SFX: SoundData = include_wav!("sfx/mark.wav");
pub(crate) static LINK_SFX: SoundData = include_wav!("sfx/static.wav");
pub(crate) static FALL_SFX: SoundData = include_wav!("sfx/fall.wav");
pub(crate) static CLEAR_SFX: SoundData = include_wav!("sfx/clear.wav");

pub(crate) fn tone(mixer: &mut agb::sound::mixer::Mixer, data: SoundData) {
    let mut channel = SoundChannel::new_high_priority(data);
    channel.volume(num!(0.85));
    let _ = mixer.play_sound(channel);
}

/// Low priority, so a cue can never take the music's channel.
pub(crate) fn cue(mixer: &mut agb::sound::mixer::Mixer, data: SoundData, volume: Num<i16, 8>) {
    let mut channel = SoundChannel::new(data);
    channel.volume(volume);
    let _ = mixer.play_sound(channel);
}

pub(crate) fn play_cues(mixer: &mut agb::sound::mixer::Mixer, world: &mut World) {
    if world.sfx & SFX_GUN != 0 {
        cue(mixer, GUN_SFX, num!(0.8));
    }
    if world.sfx & SFX_WHISTLE != 0 {
        cue(mixer, WHISTLE_SFX, num!(0.4));
    }
    if world.sfx & SFX_MARK != 0 {
        cue(mixer, MARK_SFX, num!(0.7));
    }
    if world.sfx & SFX_LINK != 0 {
        cue(mixer, LINK_SFX, num!(0.5));
    }
    if world.sfx & SFX_FALL != 0 {
        cue(mixer, FALL_SFX, num!(0.75));
    }
    if world.sfx & SFX_BOOM != 0 {
        cue(mixer, BOOM_SFX, num!(0.75));
    }
    world.sfx = 0;
}

fn start_loop(
    mixer: &mut agb::sound::mixer::Mixer,
    slot: &mut Option<ChannelId>,
    data: SoundData,
    volume: Num<i16, 8>,
) {
    let playing = match slot {
        Some(id) => mixer.channel(id).is_some(),
        None => false,
    };
    if !playing {
        let mut music = SoundChannel::new_high_priority(data);
        music.should_loop();
        music.volume(volume);
        *slot = mixer.play_sound(music);
    }
}

/// Menu and the opening cards keep the tune. Flight is the field instead.
pub(crate) fn music_bed(mixer: &mut agb::sound::mixer::Mixer, slot: &mut Option<ChannelId>, play: bool) {
    if play {
        start_loop(mixer, slot, MUSIC, num!(0.5));
    } else if let Some(id) = slot.take() {
        if let Some(channel) = mixer.channel(&id) {
            channel.stop();
        }
    }
}

/// Credits. The melody alone, so nothing sits beside it.
pub(crate) fn end_bed(mixer: &mut agb::sound::mixer::Mixer, slot: &mut Option<ChannelId>) {
    start_loop(mixer, slot, END, num!(0.55));
}

/// Daytime birds, and the war going on somewhere else in the field.
/// Night glass is quiet: the card's own night, or a day card flown that way.
pub(crate) fn ambience(
    mixer: &mut agb::sound::mixer::Mixer,
    world: &World,
    shot: bool,
    blew: bool,
    sensor: Sensor,
) {
    let day = matches!(sensor, Sensor::Day);
    if day && world.age > 40 && hash(world.age, 4) % 190 == 0 {
        let bird = if hash(world.age, 2) & 1 == 0 {
            BIRD_SFX
        } else {
            BIRD2_SFX
        };
        cue(mixer, bird, num!(0.28));
    }
    let close = 72 * 72;
    let by_rig = world.trucks.iter().any(|truck| {
        truck.alive && dist2_px(world.x, world.y, truck.x, truck.y) <= close
    }) || world.tanks.iter().any(|tank| {
        tank.alive && dist2_px(world.x, world.y, tank.x, tank.y) <= close
    });
    if by_rig && hash(world.age, 21) % 75 == 0 {
        cue(mixer, ENGINE_SFX, num!(0.3));
    }
    let talk = 48 * 48;
    let by_men = world.soldiers.iter().any(|soldier| {
        soldier.alive && dist2_px(world.x, world.y, soldier.x, soldier.y) <= talk
    });
    if by_men && hash(world.age, 27) % 120 == 0 {
        cue(mixer, VOICE_SFX, num!(0.28));
    }
    let shot_every = if day { 150 } else { 80 };
    let boom_every = if day { 340 } else { 200 };
    if !shot && world.age > 40 && hash(world.age, 9) % shot_every == 0 {
        cue(mixer, FAR_SFX, num!(0.42));
    }
    if !blew && world.age > 40 && hash(world.age, 13) % boom_every == 0 {
        cue(mixer, RUMBLE_SFX, num!(0.4));
    }
}
