use super::error::ParseError;
use super::{Collection, Component};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use url::Url;
use yaml_rust::{Yaml, YamlLoader};

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

impl TryFrom<&Yaml> for AppId {
    type Error = ParseError;

    fn try_from(e: &Yaml) -> Result<Self, Self::Error> {
        Ok(e.as_str()
            .ok_or_else(|| ParseError::missing_value("id"))?
            .into())
    }
}

impl TryFrom<&Vec<Yaml>> for Collection {
    type Error = ParseError;

    fn try_from(e: &Vec<Yaml>) -> Result<Self, Self::Error> {
        let header = &e[0];
        let version = header["Version"]
            .as_str()
            .ok_or_else(|| ParseError::missing_attribute("version", "collection"))?;

        let mut collection = CollectionBuilder::new(version);

        if let Some(arch) = header["Architecture"].as_str() {
            collection = collection.architecture(arch);
        }

        if let Some(origin) = header["Origin"].as_str() {
            if !origin.is_empty() {
                collection = collection.origin(origin);
            }
        }

        let origin = header["Origin"]
            .as_str()
            .ok_or_else(|| ParseError::missing_value("Origin"))?;
        
        if let Some(media_base_url) = header["MediaBaseUrl"].as_str() {
            if !media_base_url.is_empty() {
                collection = collection.media_base_url(media_base_url);
            }
        }

        let media_base_url = header["MediaBaseUrl"]
            .as_str()
            .ok_or_else(|| ParseError::missing_value("MediaBaseUrl"))?;

        for node in e.iter().skip(1) {
            collection = collection.component(Component::try_from((origin, media_base_url, node))?);
        }
        Ok(collection.build())
    }
}

impl TryFrom<(&str, &str, &Yaml)> for Component {
    type Error = ParseError;
    fn try_from(tuple: (&str, &str, &Yaml)) -> Result<Self, Self::Error> {
        let e: &Yaml = tuple.2.try_into().unwrap();
        let baseurl: &str = tuple.1.try_into().unwrap();
        let origin: &str = tuple.0.try_into().unwrap();
        let mut component = ComponentBuilder::default();

        component = component.origin(origin);
        if let Some(kind) = e["Type"].as_str() {
            component = component.kind(
                ComponentKind::from_str(kind)
                    .map_err(|_| ParseError::invalid_value(kind, "type", "component"))?,
            );
        }

        let app_id = AppId::try_from(
            e.as_hash()
                .unwrap()
                .get(&Yaml::from_str("ID"))
                .ok_or_else(|| ParseError::missing_tag("id"))?,
        )?;

        let mut name = TranslatableString::default();
        let mut summary = TranslatableString::default();
        let mut developer_name = TranslatableString::default();
        let mut keywords = TranslatableList::default();
        let mut description = MarkupTranslatableString::default();
        for (k, v) in e.as_hash().unwrap() {
            match k.as_str().unwrap() {
                "Name" => name.add_for_yaml_element(v),
                "Summary" => summary.add_for_yaml_element(v),
                "DeveloperName" => developer_name.add_for_yaml_element(v),
                "Description" => description.add_for_yaml_element(v),
                "ProjectLicense" => {
                    component = component.project_license(License::try_from(v)?);
                }
                "Icon" => {
                    for (x, y) in v.as_hash().unwrap() {
                        let kind = x.as_str().unwrap();
                        match kind {
                            "stock" => {
                                let name = y
                                    .as_str()
                                    .ok_or_else(|| ParseError::missing_value("stock_icon"))?;
                                component = component.icon(Icon::Stock(name.to_string()));
                            }
                            "cached" => {
                                for icon in y.as_vec().unwrap() {
                                    let name = icon["name"]
                                        .as_str()
                                        .ok_or_else(|| ParseError::missing_value("icon_name"))?
                                        .to_owned();

                                    let width: Option<u32> = match icon["width"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };

                                    let height: Option<u32> = match icon["height"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };
                                    component = component.icon(Icon::Cached {
                                        path: name.into(),
                                        width,
                                        height,
                                    });
                                }
                            }
                            "remote" => {
                                for icon in y.as_vec().unwrap() {
                                    let path = icon["url"]
                                        .as_str()
                                        .ok_or_else(|| ParseError::missing_value("icon_name"))?;

                                    let width: Option<u32> = match icon["width"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };

                                    let height: Option<u32> = match icon["height"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };
                                    let url = format!("{}{}", baseurl, path);
                                    component = component.icon(Icon::Remote {
                                        url: Url::parse(&url)?,
                                        width,
                                        height,
                                    });
                                }
                            }
                            _ => {
                                for icon in y.as_vec().unwrap() {
                                    let name = icon["name"]
                                        .as_str()
                                        .ok_or_else(|| ParseError::missing_value("icon_name"))?
                                        .to_owned();

                                    let width: Option<u32> = match icon["width"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };

                                    let height: Option<u32> = match icon["height"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };
                                    component = component.icon(Icon::Local {
                                        path: name.into(),
                                        width,
                                        height,
                                    });
                                }
                            }
                        }
                    }
                }
                "ProjectGroup" => {
                    let project_group = v
                        .as_str()
                        .ok_or_else(|| ParseError::missing_value("project_group"))?;
                    component = component.project_group(project_group.as_ref());
                }
                "CompulsoryForDesktop" => {
                    let compulsory = v
                        .as_str()
                        .ok_or_else(|| ParseError::missing_value("compulsory_for_desktop"))?;
                    component = component.compulsory_for_desktop(compulsory.as_ref());
                }
                "Package" => {
                    let pkgname = v
                        .as_str()
                        .ok_or_else(|| ParseError::missing_value("pkgname"))?;
                    component = component.pkgname(pkgname.as_ref());
                }
                "Categories" => {
                    for x in v.as_vec().unwrap() {
                        let category = x
                            .as_str()
                            .ok_or_else(|| ParseError::missing_value("category"))?
                            .to_string();
                        component =
                            component.category(Category::from_str(&category).map_err(|_| {
                                ParseError::invalid_value(&category, "$value", "category")
                            })?);
                    }
                }
                "SourcePackage" => {
                    let source_pkgname = v
                        .as_str()
                        .ok_or_else(|| ParseError::missing_value("source_pkgname"))?;
                    component = component.source_pkgname(source_pkgname.as_ref());
                }
                "Keywords" => keywords.add_for_yaml_element(v),
                "Screenshots" => {
                    for child in v.as_vec().unwrap() {
                        let mut s = ScreenshotBuilder::default().set_default(false);
                        let mut caption = TranslatableString::default();
                        for (x, y) in child.as_hash().unwrap() {
                            let kind = x.as_str().unwrap();
                            match kind {
                                "default" => {
                                    s = s.set_default(y.as_bool().unwrap_or_else(|| false));
                                }
                                "caption" => {
                                    caption.add_for_yaml_element(y);
                                }
                                "thumbnails" => {
                                    for thumbnail in y.as_vec().unwrap() {
                                        let path = thumbnail["url"].as_str().ok_or_else(|| {
                                            ParseError::missing_value("icon_name")
                                        })?;

                                        let width: Option<u32> = match thumbnail["width"].as_i64() {
                                            Some(w) => u32::try_from(w).ok(),
                                            _ => None,
                                        };

                                        let height: Option<u32> = match thumbnail["height"].as_i64()
                                        {
                                            Some(w) => u32::try_from(w).ok(),
                                            _ => None,
                                        };

                                        let url = format!("{}{}", baseurl, path);
                                        let mut img = ImageBuilder::new(Url::parse(&url)?);
                                        img = img.kind(ImageKind::Thumbnail);
                                        img = img.width(width.unwrap());
                                        img = img.height(height.unwrap());
                                        s = s.image(img.build());
                                    }
                                }
                                "source-image" => {
                                    let path = y["url"]
                                        .as_str()
                                        .ok_or_else(|| ParseError::missing_value("icon_name"))?;

                                    let width: Option<u32> = match y["width"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };

                                    let height: Option<u32> = match y["height"].as_i64() {
                                        Some(w) => u32::try_from(w).ok(),
                                        _ => None,
                                    };

                                    let url = format!("{}{}", baseurl, path);
                                    let mut img = ImageBuilder::new(Url::parse(&url)?);
                                    img = img.kind(ImageKind::Source);
                                    img = img.width(width.unwrap());
                                    img = img.height(height.unwrap());
                                    s = s.image(img.build());
                                }
                                _ => {}
                            }
                        }
                        s = s.caption(caption);
                        component = component.screenshot(s.build());
                    }
                }

                "Releases" => {
                    for x in v.as_vec().unwrap() {
                        let version = x["version"]
                            .as_str()
                            .ok_or_else(|| ParseError::missing_value("version"))?
                            .to_owned();

                        let mut release = ReleaseBuilder::new(&version);

                        let date = x["date"].as_i64().map(|d| {
                            deserialize_date(d.to_string().as_str()).map_err(|_| ParseError::invalid_value(d.to_string().as_str(), "date", "release"))
                        });

                        if let Some(d) = date {
                            release = release.date(d?);
                        }

                        let timestamp = x["unix-timestamp"].as_i64().map(|d| {
                            deserialize_date(d.to_string().as_str()).map_err(|_| ParseError::invalid_value(d.to_string().as_str(), "date", "release"))
                        });

                        if let Some(d) = timestamp {
                            release = release.date(d?);
                        }

                        if let Some(kind) = x["type"].as_str() {
                            let kind = ReleaseKind::from_str(kind)
                                .map_err(|_| ParseError::invalid_value(kind, "type", "release"))?;
                            release = release.kind(kind);
                        }

                        component = component.release(release.build())
                    }
                }
                "Extends" => {
                    for x in v.as_vec().unwrap() {
                        component = component.extend(AppId::try_from(x)?);
                    }
                }
                // "translation" => {
                //     component = component.translation(Translation::try_from(e)?);
                // }
                // "launchable" => {
                //     component = component.launchable(Launchable::try_from(e)?);
                // }
                // "content_rating" => {
                //     component = component.content_rating(ContentRating::try_from(e)?);
                // }
                // "languages" => {
                //     for child in e.children.iter() {
                //         component = component.language(Language::try_from(
                //             child
                //                 .as_element()
                //                 .ok_or_else(|| ParseError::invalid_tag("languages"))?,
                //         )?);
                //     }
                // }
                // "provides" => {
                //     for child in e.children.iter() {
                //         component = component.provide(Provide::try_from(
                //             child
                //                 .as_element()
                //                 .ok_or_else(|| ParseError::invalid_tag("prorivdes"))?,
                //         )?);
                //     }
                // }
                // "url" => {
                //     component = component.url(ProjectUrl::try_from(e)?);
                // }
                // "bundle" => {
                //     component = component.bundle(Bundle::try_from(e)?);
                // }
                // "suggests" => {
                //     for child in e.children.iter() {
                //         component = component.suggest(AppId::try_from(
                //             child
                //                 .as_element()
                //                 .ok_or_else(|| ParseError::invalid_tag("id"))?,
                //         )?);
                //     }
                // }
                // "metadata" => {
                //     for child in &e.children {
                //         let child = child
                //             .as_element()
                //             .ok_or_else(|| ParseError::invalid_tag("value"))?
                //             .to_owned();

                //         let key = child
                //             .attributes
                //             .get("key")
                //             .ok_or_else(|| ParseError::missing_attribute("key", "value"))?
                //             .to_owned();

                //         let value = child.get_text().map(|c| c.to_string());
                //         component = component.metadata(key, value);
                //     }
                // }
                // "requires" => {
                //     for child in e.children.iter() {
                //         component = component.require(AppId::try_from(
                //             child
                //                 .as_element()
                //                 .ok_or_else(|| ParseError::invalid_tag("id"))?,
                //         )?);
                //     }
                // }
                _ => (),
            }
        }
        component = component
            .name(name)
            .summary(summary)
            .keywords(keywords)
            .description(description)
            .developer_name(developer_name)
            .id(app_id);
        Ok(component.build())
    }
}

impl TryFrom<&Yaml> for License {
    type Error = ParseError;

    fn try_from(e: &Yaml) -> Result<Self, Self::Error> {
        Ok(e.as_str()
            .ok_or_else(|| ParseError::missing_value("license"))?
            .into())
    }
}
