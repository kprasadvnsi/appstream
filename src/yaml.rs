use super::error::ParseError;
use super::{Collection, Component};
use std::convert::TryFrom;
use std::str::FromStr;
use url::Url;
use yaml_rust::{YamlLoader, Yaml};

use super::builders::{
    ArtifactBuilder, CollectionBuilder, ComponentBuilder, ImageBuilder, ReleaseBuilder,
    ScreenshotBuilder, VideoBuilder,
};
use super::enums::{
    ArtifactKind, Bundle, Category, Checksum, ComponentKind, ContentAttribute,
    ContentRatingVersion, ContentState, FirmwareKind, Icon, ImageKind, Kudo, Launchable,
    ProjectUrl, Provide, ReleaseKind, ReleaseUrgency, Size, Translation,
};
use super::{
    AppId, Artifact, ContentRating, Image, Language, License, MarkupTranslatableString, Release,
    Screenshot, TranslatableList, TranslatableString, Video,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};

fn deserialize_date(date: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    Utc.datetime_from_str(&date, "%s").or_else(
        |_: chrono::ParseError| -> Result<DateTime<Utc>, chrono::ParseError> {
            let date: NaiveDateTime =
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")?.and_hms(0, 0, 0);
            Ok(DateTime::<Utc>::from_utc(date, Utc))
        },
    )
}

impl TryFrom<&Vec<Yaml>> for Collection {
    type Error = ParseError;

    fn try_from(e: &Vec<Yaml>) -> Result<Self, Self::Error> {
        let header = &e[0];
        let version = header["Version"]
        .as_str()
        .ok_or_else(|| ParseError::missing_attribute("version", "collection"))?;



        let mut collection = CollectionBuilder::new(version);
        
        if let Some(arch) = header["architecture"].as_str() {
            collection = collection.architecture(arch);
        }
        
        if let Some(origin) = header["origin"].as_str() {
            if !origin.is_empty() {
                collection = collection.origin(origin);
            }
        }

        // for node in &e.children {
        //     if let xmltree::XMLNode::Element(ref e) = node {
        //         if &*e.name == "component" {
        //             collection = collection.component(Component::try_from(e)?);
        //         }
        //     }
        // }
        Ok(collection.build())
    }
}