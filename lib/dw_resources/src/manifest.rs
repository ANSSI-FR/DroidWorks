//! The Android Manifest data structure and accessors.
//!
//! Manifest is usually accessed as part of an Android application. Thus, this
//! structure can be obtain from a loaded package.

use crate::errors::{ResourcesError, ResourcesResult};
use crate::parsers::parse_xml;
use crate::resources::Resources;
use crate::utils::{extract_single_bool_attribute, extract_single_string_attribute};
use crate::values::{ResolvedValue, Value};
use crate::writers::write_xml;
use crate::xpath;
use crate::Xml;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

/// The Manifest data structure.
///
/// This is a wrapper around [`Xml`] structure since Android Manifest is
/// reprensented by a XML file. The wrapper allows to define specific
/// accessors (and remove functions) for commonly used manifest nodes.
#[derive(Debug)]
pub struct Manifest {
    xml: Xml,
}

pub fn parse(input: &[u8]) -> ResourcesResult<Manifest> {
    let xml = parse_xml(input)?;
    Ok(Manifest { xml })
}

pub fn write(manifest: &Manifest) -> ResourcesResult<Vec<u8>> {
    write_xml(&manifest.xml)
}

impl fmt::Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.xml)
    }
}

impl Manifest {
    /// Returns the package name found in the manifest.
    /// Equivalent of xpath `/manifest@package` selection.
    pub fn package(&self) -> ResourcesResult<Option<String>> {
        let attrs = xpath::Context::new(&self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Attr("package"))?
            .attributes()?;
        if let [attribute] = attrs[..] {
            if let Value::String(s) = &attribute.typed_value {
                let val = self.xml.string_pool.get(*s)?.string()?;
                Ok(Some(val))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Returns the compileSdkVersion name found in the manifest.
    /// Equivalent of xpath `/manifest@compilesdkversion` selection.
    pub fn compile_sdk_version(&self) -> ResourcesResult<Option<u32>> {
        let attrs = xpath::Context::new(&self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Attr("compileSdkVersion"))?
            .attributes()?;
        if let [attribute] = attrs[..] {
            if let Value::IntDec(i) = &attribute.typed_value {
                Ok(Some(*i))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn version_code(&self) -> ResourcesResult<Option<u32>> {
        let attrs = xpath::Context::new(&self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Attr("versionCode"))?
            .attributes()?;
        if let [attribute] = attrs[..] {
            if let Value::IntDec(i) = &attribute.typed_value {
                Ok(Some(*i))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn version_name(&self) -> ResourcesResult<Option<String>> {
        let attrs = xpath::Context::new(&self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Attr("versionName"))?
            .attributes()?;
        if let [attribute] = attrs[..] {
            if let Value::String(s) = &attribute.typed_value {
                let val = self.xml.string_pool.get(*s)?.string()?;
                Ok(Some(val))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Search for the `allowBackup` attribute of the application node.
    /// For value return details, see method [`Self::debuggable`].
    /// This is the equivalent of xpath `/manifest/application@allowBackup`.
    pub fn allow_backup(&self, resources: Option<&Resources>) -> ResourcesResult<Option<bool>> {
        self.application_bool("allowBackup", resources)
    }

    pub fn set_allow_backup(&mut self, value: Option<bool>) -> ResourcesResult<()> {
        self.set_application_bool("allowBackup", value)
    }

    pub fn allow_clear_user_data(
        &self,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Option<bool>> {
        self.application_bool("allowClearUserData", resources)
    }

    pub fn set_allow_clear_user_data(&mut self, value: Option<bool>) -> ResourcesResult<()> {
        self.set_application_bool("allowClearUserData", value)
    }

    /// Search for the `debuggable` attribute of the application node, and returns:
    ///  - `Ok(Some(true))` if the application is declared as being debuggable,
    ///  - `Ok(Some(false))` if the application is declared as begin not debuggable,
    ///  - `Ok(None)` if one can not simply retrieve the boolean value (due to resources
    ///     indirections for example),
    ///  - `Err(err)` if `err` happened when trying to find and read the value of the attribute.
    /// This is the equivalent of xpath `/manifest/application@debuggable`.
    pub fn debuggable(&self, resources: Option<&Resources>) -> ResourcesResult<Option<bool>> {
        self.application_bool("debuggable", resources)
    }

    pub fn set_debuggable(&mut self, value: Option<bool>) -> ResourcesResult<()> {
        self.set_application_bool("debuggable", value)
    }

    pub fn uses_cleartext_traffic(
        &self,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Option<bool>> {
        self.application_bool("usesCleartextTraffic", resources)
    }

    pub fn set_uses_cleartext_traffic(&mut self, value: Option<bool>) -> ResourcesResult<()> {
        self.set_application_bool("usesCleartextTraffic", value)
    }

    fn application_bool(
        &self,
        attr_name: &str,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Option<bool>> {
        let attrs = xpath::Context::new(&self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?
            .select(xpath::Select::Attr(attr_name))?
            .attributes()?;
        extract_single_bool_attribute(&attrs, &self.xml, resources)
    }

    fn set_application_bool(
        &mut self,
        attr_name: &str,
        attr_value: Option<bool>,
    ) -> ResourcesResult<()> {
        let app_query = xpath::ContextMut::new(&mut self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?;
        if app_query.has_empty_selection() {
            return Err(ResourcesError::Structure(
                "/manifest/application not found".to_string(),
            ));
        }
        if let Some(b) = attr_value {
            let attr_query = app_query.select(xpath::Select::Attr(attr_name))?;
            if attr_query.has_empty_selection() {
                let app_query = xpath::ContextMut::new(&mut self.xml)
                    .select(xpath::Select::Root(
                        &Regex::new("^manifest$").expect("regex"),
                    ))?
                    .select(xpath::Select::Root(
                        &Regex::new("^application$").expect("regex"),
                    ))?;
                app_query.insert_attribute(attr_name.to_string(), Value::IntBoolean(b))
            } else {
                attr_query.edit_attribute(Value::IntBoolean(b))
            }
        } else {
            let attr_query = app_query.select(xpath::Select::Attr(attr_name))?;
            let _ = attr_query.remove_attributes();
            Ok(())
        }
    }

    pub fn network_security_config(
        &self,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Option<String>> {
        let attrs = xpath::Context::new(&self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?
            .select(xpath::Select::Attr("networkSecurityConfig"))?
            .attributes()?;
        extract_single_string_attribute(&attrs, &self.xml, resources)
    }

    pub fn remove_network_security_config(&mut self) -> ResourcesResult<bool> {
        let query = xpath::ContextMut::new(&mut self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?
            .select(xpath::Select::Attr("networkSecurityConfig"))?;

        query.remove_attributes()
    }

    pub fn insert_network_security_config(&mut self, filename: &str) -> ResourcesResult<()> {
        let (string_pool_index, _) = self.xml.string_pool.get_or_push(filename.to_string())?;
        let app_query = xpath::ContextMut::new(&mut self.xml)
            .select(xpath::Select::Root(
                &Regex::new("^manifest$").expect("regex"),
            ))?
            .select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?;
        app_query.insert_attribute(
            "networkSecurityConfig".to_string(),
            Value::String(string_pool_index),
        )
    }

    pub fn uses_sdk(&self, resources: Option<&Resources>) -> ResourcesResult<Vec<ManifestTag>> {
        let default_attributes = HashMap::from([
            ("minSdkVersion".to_string(), Some(ResolvedValue::Int(1))),
            ("targetSdkVersion".to_string(), None),
            ("maxSdkVersion".to_string(), None),
        ]);
        self.manifest_tags(
            false,
            &Regex::new("^uses-sdk").expect("regex"),
            default_attributes,
            resources,
        )
    }

    /// Returns a vec of permissions names declared in the manifest.
    /// This is the equivalent of xpath `/manifest/uses-permission@name` selection,
    /// and also captures `/manifest/uses-permissions-use-sdk23@name` selection.
    pub fn uses_permissions(
        &self,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Vec<ManifestTag>> {
        let default_attributes = HashMap::from([("name".to_string(), None)]);
        self.manifest_tags(
            false,
            &Regex::new("^uses-permission").expect("regex"),
            default_attributes,
            resources,
        )
    }

    /// Adds permission nodes by name.
    pub fn add_uses_permission(&mut self, permission: &str) -> ResourcesResult<()> {
        let (permission_index, _) = self.xml.string_pool.get_or_push(permission.to_string())?;

        let query = xpath::ContextMut::new(&mut self.xml).select(xpath::Select::Root(
            &Regex::new("^manifest$").expect("regex"),
        ))?;

        let attrs = vec![(
            Some("android".to_string()),
            "name".to_string(),
            Value::String(permission_index),
        )];

        let _ = query.add_self_contained_nodes("uses-permission".to_string(), attrs)?;

        Ok(())
    }

    /// Removes permission nodes by name. In xpath terms, removes the
    /// `/manifest/uses-permission[@name=permission]` nodes.
    pub fn remove_uses_permission(&mut self, permission: &str) -> ResourcesResult<bool> {
        self.remove_tags_by_name(
            false,
            &Regex::new("^uses-permission").expect("regex"),
            permission,
        )
    }

    /// Returns a vec of features names declared in the manifest.
    /// This is the equivalent of xpath `/manifest/uses-feature@name` selection.
    pub fn uses_features(
        &self,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Vec<ManifestTag>> {
        let default_attributes = HashMap::from([
            ("name".to_string(), None),
            ("required".to_string(), Some(ResolvedValue::Bool(true))),
        ]);
        self.manifest_tags(
            false,
            &Regex::new("^uses-feature").expect("regex"),
            default_attributes,
            resources,
        )
    }

    /// Removes feature nodes by name. In xpath terms, removes the
    /// `/manifest/uses-feature[@name=feature]` nodes.
    pub fn remove_uses_feature(&mut self, feature: &str) -> ResourcesResult<bool> {
        self.remove_tags_by_name(false, &Regex::new("^uses-feature").expect("regex"), feature)
    }

    /// Returns a vec of activities names declared in the manifest.
    /// This is the equivalent of xpath `/manifest/application/activity@name` selection,
    /// and also captures `/manifest/application/activity-alias@name` selection.
    pub fn activities(&self, resources: Option<&Resources>) -> ResourcesResult<Vec<ManifestTag>> {
        let default_attributes = HashMap::from([
            ("name".to_string(), None),
            ("enabled".to_string(), Some(ResolvedValue::Bool(true))),
            ("exported".to_string(), None),
        ]);

        let mut tags = self.manifest_tags(
            true,
            &Regex::new("^activity").expect("regex"),
            default_attributes,
            resources,
        )?;

        for tag in &mut tags {
            let exported = tag.attributes.get_mut(&"exported".to_string()).unwrap();
            if exported.is_none() {
                *exported = Some(ResolvedValue::Bool(tag.has_intent_filter));
            }
        }

        Ok(tags)
    }

    /// Removes activity nodes by name. In xpath terms, removes the
    /// `/manifest/activity[@name=activity]` nodes.
    pub fn remove_activity(&mut self, activity: &str) -> ResourcesResult<bool> {
        self.remove_tags_by_name(true, &Regex::new("^activity").expect("regex"), activity)
    }

    /// Returns a vec of services names declared in the manifest.
    /// This is the equivalent of xpath `/manifest/application/service@name` selection.
    pub fn services(&self, resources: Option<&Resources>) -> ResourcesResult<Vec<ManifestTag>> {
        let default_attributes = HashMap::from([
            ("name".to_string(), None),
            ("enabled".to_string(), Some(ResolvedValue::Bool(true))),
            ("exported".to_string(), None),
        ]);

        let mut tags = self.manifest_tags(
            true,
            &Regex::new("^service").expect("regex"),
            default_attributes,
            resources,
        )?;

        for tag in &mut tags {
            let exported = tag.attributes.get_mut(&"exported".to_string()).unwrap();
            if exported.is_none() {
                *exported = Some(ResolvedValue::Bool(tag.has_intent_filter));
            }
        }

        Ok(tags)
    }

    /// Removes service nodes by name. In xpath terms, removes the
    /// `/manifest/service[@name=service]` nodes.
    pub fn remove_service(&mut self, service: &str) -> ResourcesResult<bool> {
        self.remove_tags_by_name(true, &Regex::new("^service").expect("regex"), service)
    }

    /// Returns a vec of receivers names declared in the manifest.
    /// This is the equivalent of xpath `/manifest/application/receiver@name` selection.
    pub fn receivers(&self, resources: Option<&Resources>) -> ResourcesResult<Vec<ManifestTag>> {
        let default_attributes = HashMap::from([
            ("name".to_string(), None),
            ("enabled".to_string(), Some(ResolvedValue::Bool(true))),
            ("exported".to_string(), Some(ResolvedValue::Bool(true))),
        ]);

        let mut tags = self.manifest_tags(
            true,
            &Regex::new("^receiver").expect("regex"),
            default_attributes,
            resources,
        )?;

        for tag in &mut tags {
            let exported = tag.attributes.get_mut(&"exported".to_string()).unwrap();
            if exported.is_none() {
                *exported = Some(ResolvedValue::Bool(tag.has_intent_filter));
            }
        }

        Ok(tags)
    }

    /// Removes receiver nodes by name. In xpath terms, removes the
    /// `/manifest/receiver[@name=receiver]` nodes.
    pub fn remove_receiver(&mut self, receiver: &str) -> ResourcesResult<bool> {
        self.remove_tags_by_name(true, &Regex::new("^receiver").expect("regex"), receiver)
    }

    /// Returns a vec of providers names declared in the manifest.
    /// This is the equivalent of xpath `/manifest/application/provider@name` selection.
    pub fn providers(&self, resources: Option<&Resources>) -> ResourcesResult<Vec<ManifestTag>> {
        let mut min_sdk = 0;
        let sdks = self.uses_sdk(resources)?;
        for mut sdk in sdks {
            let oattr = sdk
                .attributes
                .remove(&"minSdkVersion".to_string())
                .flatten();

            if let Some(attr) = oattr {
                match attr {
                    ResolvedValue::Int(s) => {
                        if s < min_sdk {
                            min_sdk = s
                        }
                    }
                    _ => {
                        return Err(ResourcesError::UnexpectedValue {
                            name: "minSdkVersion".to_string(),
                            typ: "Int".to_string(),
                        })
                    }
                }
            }
        }

        let default_attributes = HashMap::from([
            ("name".to_string(), None),
            ("enabled".to_string(), Some(ResolvedValue::Bool(true))),
            (
                "exported".to_string(),
                Some(ResolvedValue::Bool(min_sdk <= 16)),
            ),
        ]);
        self.manifest_tags(
            true,
            &Regex::new("^provider").expect("regex"),
            default_attributes,
            resources,
        )
    }

    /// Removes receiver nodes by name. In xpath terms, removes the
    /// `/manifest/provider[@name=provider]` nodes.
    pub fn remove_provider(&mut self, provider: &str) -> ResourcesResult<bool> {
        self.remove_tags_by_name(true, &Regex::new("^provider").expect("regex"), provider)
    }

    fn remove_tags_by_name(
        &mut self,
        in_application: bool,
        tag: &Regex,
        name: &str,
    ) -> ResourcesResult<bool> {
        let mut query = xpath::ContextMut::new(&mut self.xml).select(xpath::Select::Root(
            &Regex::new("^manifest$").expect("regex"),
        ))?;

        if in_application {
            query = query.select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?;
        }

        query = query
            .select(xpath::Select::Root(tag))?
            .filter(xpath::Predicate::Attr("name", name))?;

        query.remove_nodes()
    }

    fn manifest_tags(
        &self,
        in_application: bool,
        tag: &Regex,
        default_attributes: HashMap<String, Option<ResolvedValue>>,
        resources: Option<&Resources>,
    ) -> ResourcesResult<Vec<ManifestTag>> {
        let mut tags = xpath::Context::new(&self.xml).select(xpath::Select::Root(
            &Regex::new("^manifest$").expect("regex"),
        ))?;

        if in_application {
            tags = tags.select(xpath::Select::Root(
                &Regex::new("^application$").expect("regex"),
            ))?;
        }

        tags = tags.select(xpath::Select::Root(tag))?;
        let query = tags.clone();
        let attrs = tags.nodes()?;

        attrs
            .into_iter()
            .map(|(_, attrs)| {
                let mut attributes = default_attributes.clone();

                for attr in &attrs.attrs {
                    let attr_name = self.xml.string_pool.get(attr.name)?.string()?;
                    if attributes.contains_key(attr_name.as_str()) {
                        match attr.typed_value.resolve(&self.xml.string_pool, resources) {
                            Ok(value) => attributes.insert(attr_name, Some(value)),
                            Err(ResourcesError::TooComplexResource(_)) => {
                                attributes.insert(attr_name, None)
                            }
                            Err(other_err) => return Err(other_err),
                        };
                    }
                }

                let has_intent_filter = !query
                    .clone()
                    .select(xpath::Select::Root(
                        &Regex::new("^intent-filter$").expect("regex"),
                    ))?
                    .nodes()?
                    .is_empty();

                Ok(ManifestTag {
                    attributes,
                    has_intent_filter,
                })
            })
            .collect()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ManifestTag {
    pub attributes: HashMap<String, Option<ResolvedValue>>,
    #[serde(skip)]
    has_intent_filter: bool,
}

impl ManifestTag {
    #[must_use]
    pub fn name(&self) -> Option<String> {
        let value = self.attributes.get("name")?.clone()?;
        value.as_str().map(std::string::ToString::to_string)
    }
}
