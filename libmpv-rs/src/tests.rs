// Copyright (C) 2016  ParadoxSpiral
//
// This file is part of libmpv-rs.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 2.1 of the License, or (at your option) any later version.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA

use events::EventContextExt;

use crate::events::{Event, PropertyData};
use crate::node::{MpvNode, MpvNodeRef, MpvNodeValue};
use crate::*;

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

static TEST_FILE_PATH: LazyLock<String> = LazyLock::new(|| {
    std::fs::canonicalize("test-data/speech_12kbps_mb.wav")
        .expect("canonicalizing test file")
        .into_os_string()
        .into_string()
        .expect("converting path to utf8 str")
});

static TEST_FILE_PATHC: LazyLock<CString> =
    LazyLock::new(|| CString::new(TEST_FILE_PATH.clone()).expect("converting path to cstr"));

fn get_mpv() -> Mpv {
    Mpv::with_initializer(|init| -> Result<()> {
        init.set_option(c"vo", c"null")?;
        init.set_option(c"ao", c"null")
    })
    .expect("initialization failed")
}

#[cfg_attr(miri, ignore)]
#[test]
fn initializer() {
    let mpv = Mpv::with_initializer(|init| -> Result<()> {
        init.set_option(c"osc", true)?;
        init.set_option(c"input-default-bindings", true)?;
        init.set_option(c"volume", 30)?;
        Ok(())
    })
    .unwrap();

    assert_eq!(true, mpv.get_property("osc").unwrap());
    assert_eq!(true, mpv.get_property("input-default-bindings").unwrap());
    assert_eq!(30i64, mpv.get_property("volume").unwrap());
}
#[cfg_attr(miri, ignore)]
#[test]
fn test_file_exists() {
    assert!(
        File::open(TEST_FILE_PATH.clone()).is_ok(),
        "Unable to open test file at test-data/speech_12kbps_mb.wav"
    )
}
#[cfg_attr(miri, ignore)]
#[test]
fn properties() {
    let mpv = get_mpv();
    mpv.set_property(c"volume", 0).unwrap();
    mpv.set_property(c"vo", c"null").unwrap();
    mpv.set_property(c"ytdl-format", c"best[width<240]")
        .unwrap();
    mpv.set_property(c"sub-gauss", 0.6).unwrap();

    assert_eq!(0i64, mpv.get_property("volume").unwrap());
    let vo: MpvStr = mpv.get_property("vo").unwrap();
    assert_eq!("null", &*vo);
    assert_eq!(true, mpv.get_property("ytdl").unwrap());
    let subg: f64 = mpv.get_property("sub-gauss").unwrap();
    assert_eq!(
        0.6,
        f64::round(subg * f64::powi(10.0, 4)) / f64::powi(10.0, 4)
    );
    mpv.playlist_replace(&TEST_FILE_PATHC).unwrap();
    thread::sleep(Duration::from_millis(250));

    let title: MpvStr = mpv.get_property("media-title").unwrap();
    assert_eq!(&*title, "speech_12kbps_mb.wav");
}

macro_rules! assert_event_occurs {
    ($ctx:ident, $timeout:literal, $( $expected:pat),+) => {
        loop {
            match $ctx.wait_event($timeout) {
                $( Some($expected) )|+ => {
                    break;
                },
                None => {
                    continue
                },
                other => panic!("Event did not occur, got: {:?}", other),
            }
        }
    }
}
#[cfg_attr(miri, ignore)]
#[test]
fn events() {
    let mut mpv = Mpv::with_initializer(|init| {
        init.set_option(c"ytdl", false)?;
        init.set_option(c"vo", c"null")?;
        init.set_option(c"ao", c"null")
    })
    .unwrap();
    mpv.disable_deprecated_events().unwrap();

    mpv.observe_property("volume", Format::Int64, 0).unwrap();
    mpv.observe_property("media-title", Format::String, 1)
        .unwrap();

    mpv.set_property(c"vo", c"null").unwrap();

    assert_event_occurs!(
        mpv,
        3.,
        Ok(Event::PropertyChange {
            name: "volume",
            change: PropertyData::Int64(100),
            reply_userdata: 0,
        })
    );

    mpv.set_property(c"volume", 0).unwrap();
    assert_event_occurs!(
        mpv,
        10.,
        Ok(Event::PropertyChange {
            name: "volume",
            change: PropertyData::Int64(0),
            reply_userdata: 0,
        })
    );
    assert!(mpv.wait_event(3.).is_none());

    mpv.playlist_append_play(c"https://www.youtube.com/watch?v=DLzxrzFCyOs")
        .unwrap();
    assert_event_occurs!(
        mpv,
        10.,
        Ok(Event::StartFile {
            playlist_entry_id: 1
        })
    );
    assert_event_occurs!(
        mpv,
        10.,
        Ok(Event::PropertyChange {
            name: "media-title",
            change: PropertyData::Str("watch?v=DLzxrzFCyOs"),
            reply_userdata: 1,
        })
    );
    assert_event_occurs!(mpv, 20., Err(Error::Raw(_)));
    assert!(mpv.wait_event(3.).is_none());

    mpv.playlist_append_play(&TEST_FILE_PATHC).unwrap();
    assert_event_occurs!(
        mpv,
        10.,
        Ok(Event::StartFile {
            playlist_entry_id: 2
        })
    );
    assert_event_occurs!(
        mpv,
        3.,
        Ok(Event::PropertyChange {
            name: "media-title",
            change: PropertyData::Str("speech_12kbps_mb.wav"),
            reply_userdata: 1,
        })
    );
    assert_event_occurs!(mpv, 3., Ok(Event::AudioReconfig));
    assert_event_occurs!(mpv, 3., Ok(Event::AudioReconfig));
    assert_event_occurs!(mpv, 3., Ok(Event::FileLoaded));
    assert_event_occurs!(mpv, 3., Ok(Event::AudioReconfig));
    assert_event_occurs!(mpv, 3., Ok(Event::PlaybackRestart));
    assert!(mpv.wait_event(3.).is_none());
}
#[cfg_attr(miri, ignore)]
#[test]
fn node_map() -> Result<()> {
    let mpv = get_mpv();

    mpv.playlist_append_play(&TEST_FILE_PATHC)?;

    thread::sleep(Duration::from_millis(250));
    let audio_params: MpvNode = mpv.get_property("audio-params")?;
    let params: HashMap<&CStr, MpvNodeRef<'_>> = audio_params
        .as_ref()
        .to_map()
        .ok_or_else(|| Error::Null)?
        .into_iter()
        .collect();

    assert_eq!(params.len(), 5);

    let format = params.get(c"format").unwrap().value()?;
    assert!(matches!(format, MpvNodeValue::String("s16")));

    let samplerate = params.get(c"samplerate").unwrap().value()?;
    assert!(matches!(samplerate, MpvNodeValue::Int64(48_000)));

    let channels = params.get(c"channels").unwrap().value()?;
    assert!(matches!(channels, MpvNodeValue::String("mono")));

    let hr_channels = params.get(c"hr-channels").unwrap().value()?;
    assert!(matches!(hr_channels, MpvNodeValue::String("mono")));

    let channel_count = params.get(c"channel-count").unwrap().value()?;
    assert!(matches!(channel_count, MpvNodeValue::Int64(1)));

    Ok(())
}
#[cfg_attr(miri, ignore)]
#[test]
fn node_array() -> Result<()> {
    let mpv = Mpv::new()?;

    mpv.playlist_append_play(&TEST_FILE_PATHC)?;

    thread::sleep(Duration::from_millis(250));
    let playlist: MpvNode = mpv.get_property("playlist")?;
    let items: Vec<MpvNodeRef<'_>> = playlist
        .as_ref()
        .to_array()
        .ok_or_else(|| Error::Null)?
        .into_iter()
        .collect();

    assert_eq!(items.len(), 1);
    let track: HashMap<&CStr, MpvNodeRef<'_>> = items[0]
        .to_map()
        .ok_or_else(|| Error::Null)?
        .into_iter()
        .collect();

    let filename = track.get(c"filename").unwrap().value()?;

    let MpvNodeValue::String(path) = filename else {
        panic!("filename is not a string node")
    };
    assert_eq!(path, TEST_FILE_PATH.as_str());

    Ok(())
}
