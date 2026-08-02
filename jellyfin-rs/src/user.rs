use super::err::Result;
use serde::Deserialize;
use serde::Serialize;

use super::session::SessionInfo;
use crate::AuthStatus;
use crate::Authed;
use crate::JellyfinClient;
use crate::connect::JsonResponse;
use crate::request::NoQuery;
use crate::request::RequestBuilderExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserIdQuery<'id> {
    user_id: &'id str,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct User {
    pub name: String,
    pub server_id: String,
    pub server_name: Option<String>,
    pub id: String,
    pub primary_image_tag: Option<String>,
    pub has_password: bool,
    pub has_configured_password: bool,
    pub has_configured_easy_password: bool,
    pub enable_auto_login: bool,
    pub last_login_date: Option<String>,
    pub last_activity_date: Option<String>,
    pub configuration: UserConfiguration,
    pub policy: UserPolicy,
    pub primary_image_aspect_ratio: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct MediaStream {
    //         "Codec": "string",
    //         "CodecTag": "string",
    //         "Language": "string",
    //         "ColorRange": "string",
    //         "ColorSpace": "string",
    //         "ColorTransfer": "string",
    //         "ColorPrimaries": "string",
    //         "DvVersionMajor": 0,
    //         "DvVersionMinor": 0,
    //         "DvProfile": 0,
    //         "DvLevel": 0,
    //         "RpuPresentFlag": 0,
    //         "ElPresentFlag": 0,
    //         "BlPresentFlag": 0,
    //         "DvBlSignalCompatibilityId": 0,
    //         "Comment": "string",
    //         "TimeBase": "string",
    //         "CodecTimeBase": "string",
    //         "Title": "string",
    //         "VideoRange": "string",
    //         "VideoRangeType": "string",
    //         "VideoDoViTitle": "string",
    //         "LocalizedUndefined": "string",
    //         "LocalizedDefault": "string",
    //         "LocalizedForced": "string",
    //         "LocalizedExternal": "string",
    //         "DisplayTitle": "string",
    //         "NalLengthSize": "string",
    //         "IsInterlaced": true,
    //         "IsAVC": true,
    //         "ChannelLayout": "string",
    //         "BitRate": 0,
    //         "BitDepth": 0,
    //         "RefFrames": 0,
    //         "PacketLength": 0,
    //         "Channels": 0,
    //         "SampleRate": 0,
    //         "IsDefault": true,
    //         "IsForced": true,
    //         "Height": 0,
    //         "Width": 0,
    //         "AverageFrameRate": 0,
    //         "RealFrameRate": 0,
    //         "Profile": "string",
    //         "Type": "Audio",
    //         "AspectRatio": "string",
    //         "Index": 0,
    //         "Score": 0,
    //         "IsExternal": true,
    //         "DeliveryMethod": "Encode",
    //         "DeliveryUrl": "string",
    //         "IsExternalUrl": true,
    //         "IsTextSubtitleStream": true,
    //         "SupportsExternalStream": true,
    //         "Path": "string",
    //         "PixelFormat": "string",
    //         "Level": 0,
    //         "IsAnamorphic": true
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct MediaSource {
    pub protocol: String,
    pub id: String,
    pub path: String,
    pub encoder_path: Option<String>,
    pub encoder_protocol: Option<String>,
    pub r#type: String,
    pub container: String,
    pub size: i64,
    pub name: String,
    pub is_remote: bool,
    pub etag: Option<String>,
    pub run_time_ticks: i64,
    pub read_at_native_framerate: bool,
    pub ignore_dts: bool,
    pub ignore_index: bool,
    pub gen_pts_input: bool,
    pub supports_transcoding: bool,
    pub supports_direct_stream: bool,
    pub supports_direct_play: bool,
    pub is_infinite_stream: bool,
    pub requires_opening: bool,
    pub open_token: Option<String>,
    pub requires_closing: bool,
    pub live_stream_id: Option<String>,
    pub buffer_ms: Option<i64>,
    pub requires_looping: bool,
    pub supports_probing: bool,
    pub video_type: String,
    pub iso_type: Option<String>,
    pub video_3d_format: Option<String>,
    pub media_streams: Vec<MediaStream>,
    // media_attachments: Vec<MediaAttachment>,
    //     "MediaAttachments": [
    //       {
    //         "Codec": "string",
    //         "CodecTag": "string",
    //         "Comment": "string",
    //         "Index": 0,
    //         "FileName": "string",
    //         "MimeType": "string",
    //         "DeliveryUrl": "string"
    //       }
    //     ],
    pub formats: Vec<String>,
    pub bitrate: i64,
    pub timestamp: Option<String>,
    // required_http_headers: serde_json::Map<String, serde_json::Value>,
    pub transcoding_url: Option<String>,
    pub transcoding_sub_protocol: Option<String>,
    pub transcoding_container: Option<String>,
    pub analyze_duration_ms: Option<i64>,
    pub default_audio_stream_index: Option<i64>,
    pub default_subtitle_stream_index: Option<i64>,
    //     "Formats": [
    //       "string"
    //     ],
    //     "RequiredHttpHeaders": {
    //       "property1": "string",
    //       "property2": "string"
    //     },
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct UserConfiguration {
    pub audio_language_preference: Option<String>,
    pub play_default_audio_track: bool,
    pub subtitle_language_preference: String,
    pub display_missing_episodes: bool,
    pub grouped_folders: Vec<String>,
    pub subtitle_mode: String,
    pub display_collections_view: bool,
    pub enable_local_password: bool,
    pub ordered_views: Vec<String>,
    pub latest_items_excludes: Vec<String>,
    pub my_media_excludes: Vec<String>,
    pub hide_played_in_latest: bool,
    pub remember_audio_selections: bool,
    pub remember_subtitle_selections: bool,
    pub enable_next_episode_auto_play: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct UserPolicy {
    pub is_administrator: bool,
    pub is_hidden: bool,
    pub is_disabled: bool,
    pub max_parental_rating: Option<i64>,
    pub blocked_tags: Vec<String>,
    pub enable_user_preference_access: bool,
    pub access_schedules: Vec<UserAccessSchedule>,
    pub block_unrated_items: Vec<String>,
    pub enable_remote_control_of_other_users: bool,
    pub enable_shared_device_control: bool,
    pub enable_remote_access: bool,
    pub enable_live_tv_management: bool,
    pub enable_live_tv_access: bool,
    pub enable_media_playback: bool,
    pub enable_audio_playback_transcoding: bool,
    pub enable_video_playback_transcoding: bool,
    pub enable_playback_remuxing: bool,
    pub force_remote_source_transcoding: bool,
    pub enable_content_deletion: bool,
    pub enable_content_deletion_from_folders: Vec<String>,
    pub enable_content_downloading: bool,
    pub enable_sync_transcoding: bool,
    pub enable_media_conversion: bool,
    pub enabled_devices: Vec<String>,
    pub enable_all_devices: bool,
    pub enabled_channels: Vec<String>,
    pub enable_all_channels: bool,
    pub enabled_folders: Vec<String>,
    pub enable_all_folders: bool,
    pub invalid_login_attempt_count: i64,
    pub login_attempts_before_lockout: i64,
    pub max_active_sessions: i64,
    pub enable_public_sharing: bool,
    pub blocked_media_folders: Vec<String>,
    pub blocked_channels: Vec<String>,
    pub remote_client_bitrate_limit: i64,
    pub authentication_provider_id: String,
    pub password_reset_provider_id: String,
    pub sync_play_access: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct UserAccessSchedule {
    pub user_id: String,
    pub day_of_week: String,
    pub start_hour: i64,
    pub end_hour: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserAuth {
    pub user: User,
    pub session_info: SessionInfo,
    pub access_token: String,
    pub server_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GetUsersQuery {
    is_hidden: bool,
    is_disabled: bool,
}

impl<Auth: Authed> JellyfinClient<Auth> {
    /// Gets a list of all users that the `UserAuth` has access to, given some filters.
    pub async fn get_users(
        &self,
        is_hidden: bool,
        is_disabled: bool,
    ) -> Result<JsonResponse<Vec<User>>> {
        self.send_request_json(
            self.get(
                "/Users",
                &GetUsersQuery {
                    is_hidden,
                    is_disabled,
                },
            )?
            .empty_body()?,
        )
        .await
    }
    pub async fn get_user_by_id(&self, id: impl AsRef<str>) -> Result<JsonResponse<User>> {
        self.send_request_json(
            self.get(
                |prefix: &mut String| {
                    prefix.push_str("/Users/");
                    prefix.push_str(id.as_ref());
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await
    }
    pub async fn delete_user(&self, id: impl AsRef<str>) -> Result<()> {
        self.send_request(
            self.delete(
                |prefix: &mut String| {
                    prefix.push_str("/Users/");
                    prefix.push_str(id.as_ref());
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await?;
        Ok(())
    }
    pub async fn update_user(&self, id: impl AsRef<str>, new_info: &User) -> Result<()> {
        self.send_request(
            self.post(
                "/Users",
                &UserIdQuery {
                    user_id: id.as_ref(),
                },
            )?
            .json_body(new_info)?,
        )
        .await?;
        Ok(())
    }
    pub async fn update_user_conf(
        &self,
        id: impl AsRef<str>,
        new_conf: &UserConfiguration,
    ) -> Result<()> {
        self.send_request(
            self.post(
                "/Users/Configuration",
                &UserIdQuery {
                    user_id: id.as_ref(),
                },
            )?
            .json_body(new_conf)?,
        )
        .await?;
        Ok(())
    }
    pub async fn update_user_password(
        &self,
        id: impl AsRef<str>,
        new_password: impl AsRef<str>,
    ) -> Result<()> {
        self.send_request(
            self.post(
                "/Users/Password",
                &UserIdQuery {
                    user_id: id.as_ref(),
                },
            )?
            .json_body(&NewPwReq {
                new_pw: new_password.as_ref(),
            })?,
        )
        .await?;
        Ok(())
    }
    pub async fn update_user_policy(
        &self,
        id: impl AsRef<str>,
        new_policy: &UserPolicy,
    ) -> Result<()> {
        self.send_request(
            self.post(
                |prefix: &mut String| {
                    prefix.push_str("/Users/");
                    prefix.push_str(id.as_ref());
                    prefix.push_str("/Policy");
                },
                NoQuery,
            )?
            .json_body(new_policy)?,
        )
        .await?;
        Ok(())
    }
    pub async fn get_user_by_auth(&self) -> Result<JsonResponse<User>> {
        self.send_request_json(self.get("/Users/Me", NoQuery)?.empty_body()?)
            .await
    }
    pub async fn create_user(
        &self,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<JsonResponse<User>> {
        self.send_request_json(
            self.post("/Users/New", NoQuery)?
                .json_body(&CreateUserReq {
                    name: username.as_ref(),
                    password: password.as_ref(),
                })?,
        )
        .await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct NewPwReq<'pw> {
    new_pw: &'pw str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ForgotPwReq<'s> {
    entered_username: &'s str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForgotPasswordAction {
    ContactAdmin,
    PinCode,
    InNetworkRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ForgotPasswordResponse {
    action: ForgotPasswordAction,
    pin_file: Option<String>,
    pin_expiration_date: Option<String>,
}

impl<Auth: AuthStatus> JellyfinClient<Auth> {
    pub async fn user_forgot_password(
        &self,
        username: impl AsRef<str>,
    ) -> Result<JsonResponse<ForgotPasswordResponse>> {
        self.send_request_json(self.post("/Users/ForgotPassword", NoQuery)?.json_body(
            &ForgotPwReq {
                entered_username: username.as_ref(),
            },
        )?)
        .await
    }
    pub async fn user_redeem_forgot_password_pin(
        &self,
        pin: impl AsRef<str>,
    ) -> Result<JsonResponse<RedeemForgotPasswordResponse>> {
        self.send_request_json(
            self.post("/Users/ForgotPassword/Pin", NoQuery)?
                .json_body(&RedeemForgotPasswordReq { pin: pin.as_ref() })?,
        )
        .await
    }
    pub async fn get_public_user_list(&self) -> Result<JsonResponse<Vec<User>>> {
        self.send_request_json(self.get("/Users/Public", NoQuery)?.empty_body()?)
            .await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct RedeemForgotPasswordReq<'s> {
    pin: &'s str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RedeemForgotPasswordResponse {
    success: bool,
    users_reset: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct CreateUserReq<'s> {
    name: &'s str,
    password: &'s str,
}
