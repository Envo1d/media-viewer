#[cfg(windows)]
pub fn query_video_properties(path: &str) -> VideoProperties {
    use windows::{
        core::HSTRING,
        Win32::{
            Foundation::PROPERTYKEY,
            System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
            UI::Shell::{
                IShellItem2,
                PropertiesSystem::{
                    IPropertyStore, GETPROPERTYSTOREFLAGS, GPS_BESTEFFORT, GPS_DEFAULT,
                    GPS_OPENSLOWITEM,
                },
                SHCreateItemFromParsingName,
            },
        },
    };

    // GUID for the "Media" and "Video" property namespaces.
    // These are stable Win32 constants — defined here to avoid depending on
    // the `windows-sys` bindings for property keys that are not yet exposed
    // as named constants in all versions of the `windows` crate.
    const FMTID_MEDIA: windows::core::GUID =
        windows::core::GUID::from_u128(0x64440490_4C8B_11D1_8B70_080036B11A03);
    const FMTID_VIDEO: windows::core::GUID =
        windows::core::GUID::from_u128(0x64440491_4C8B_11D1_8B70_080036B11A03);

    // PKEY_Media_Duration (pid = 3)
    const PKEY_MEDIA_DURATION: PROPERTYKEY = PROPERTYKEY {
        fmtid: FMTID_MEDIA,
        pid: 3,
    };
    // PKEY_Video_FrameWidth (pid = 3)
    const PKEY_VIDEO_FRAME_WIDTH: PROPERTYKEY = PROPERTYKEY {
        fmtid: FMTID_VIDEO,
        pid: 3,
    };
    // PKEY_Video_FrameHeight (pid = 4)
    const PKEY_VIDEO_FRAME_HEIGHT: PROPERTYKEY = PROPERTYKEY {
        fmtid: FMTID_VIDEO,
        pid: 4,
    };
    // PKEY_Video_FrameRate (pid = 6) - value is fps × 1 000
    const PKEY_VIDEO_FRAME_RATE: PROPERTYKEY = PROPERTYKEY {
        fmtid: FMTID_VIDEO,
        pid: 6,
    };

    let mut result = VideoProperties::default();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let outcome: windows::core::Result<()> = (|| {
            let item: IShellItem2 = SHCreateItemFromParsingName(&HSTRING::from(path), None)?;

            let flags =
                GETPROPERTYSTOREFLAGS(GPS_DEFAULT.0 | GPS_BESTEFFORT.0 | GPS_OPENSLOWITEM.0);

            let store: IPropertyStore = item.GetPropertyStore(flags)?;

            // Duration
            if let Ok(pv) = store.GetValue(&PKEY_MEDIA_DURATION) {
                // VT_UI8 = 21; value is 100-nanosecond units.
                let inner = &pv.Anonymous.Anonymous;
                if inner.vt.0 == 21 {
                    let hundred_ns = inner.Anonymous.uhVal;
                    result.duration_secs = Some(hundred_ns as f64 / 10_000_000.0);
                }
            }

            // Frame dimensions
            if let Ok(pv) = store.GetValue(&PKEY_VIDEO_FRAME_WIDTH) {
                let inner = &pv.Anonymous.Anonymous;
                // VT_UI4 = 19
                if inner.vt.0 == 19 {
                    result.frame_width = Some(inner.Anonymous.ulVal);
                }
            }
            if let Ok(pv) = store.GetValue(&PKEY_VIDEO_FRAME_HEIGHT) {
                let inner = &pv.Anonymous.Anonymous;
                if inner.vt.0 == 19 {
                    result.frame_height = Some(inner.Anonymous.ulVal);
                }
            }

            // Frame rate
            // Stored as VT_UI4 representing frames per 1 000 seconds.
            if let Ok(pv) = store.GetValue(&PKEY_VIDEO_FRAME_RATE) {
                let inner = &pv.Anonymous.Anonymous;
                if inner.vt.0 == 19 {
                    let raw = inner.Anonymous.ulVal;
                    if raw > 0 {
                        result.frame_rate = Some(raw as f64 / 1000.0);
                    }
                }
            }

            Ok(())
        })();

        if let Err(e) = outcome {
            tracing::error!(path = %path, ?e, "Property query failed");
        }

        CoUninitialize();
    }

    result
}

#[derive(Debug, Default, Clone)]
pub struct VideoProperties {
    pub duration_secs: Option<f64>,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
    pub frame_rate: Option<f64>,
}

// Non-Windows stub
#[cfg(not(windows))]
#[derive(Debug, Default, Clone)]
pub struct VideoProperties {
    pub duration_secs: Option<f64>,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
    pub frame_rate: Option<f64>,
}

#[cfg(not(windows))]
pub fn query_video_properties(_path: &str) -> VideoProperties {
    VideoProperties::default()
}
