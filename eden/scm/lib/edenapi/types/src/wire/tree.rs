/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use minibytes::Bytes;
#[cfg(any(test, feature = "for-tests"))]
use quickcheck::Arbitrary;
#[cfg(any(test, feature = "for-tests"))]
use quickcheck_arbitrary_derive::Arbitrary;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::SaplingRemoteApiServerError;
use crate::tree::TreeAttributes;
use crate::tree::TreeChildDirectoryEntry;
use crate::tree::TreeChildEntry;
use crate::tree::TreeChildFileEntry;
use crate::tree::TreeEntry;
use crate::tree::TreeRequest;
pub use crate::tree::WireUploadTreeEntry;
pub use crate::tree::WireUploadTreeRequest;
pub use crate::tree::WireUploadTreeResponse;
use crate::wire::ToApi;
use crate::wire::ToWire;
use crate::wire::WireFileMetadata;
use crate::wire::WireKey;
use crate::wire::WireParents;
use crate::wire::WireSaplingRemoteApiServerError;
use crate::wire::WireToApiConversionError;
use crate::wire::WireTreeAuxData;
use crate::wire::is_default;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WireTreeEntry {
    #[serde(rename = "0", default, skip_serializing_if = "is_default")]
    key: Option<WireKey>,

    #[serde(rename = "1", default, skip_serializing_if = "is_default")]
    data: Option<Bytes>,

    #[serde(rename = "2", default, skip_serializing_if = "is_default")]
    parents: Option<WireParents>,

    #[serde(rename = "3", default, skip_serializing_if = "is_default")]
    children: Option<Vec<WireTreeChildEntry>>,

    #[serde(rename = "4", default, skip_serializing_if = "is_default")]
    pub error: Option<WireSaplingRemoteApiServerError>,

    #[serde(rename = "5", default, skip_serializing_if = "is_default")]
    pub tree_aux_data: Option<WireTreeAuxData>,
}

impl ToWire for Result<TreeEntry, SaplingRemoteApiServerError> {
    type Wire = WireTreeEntry;

    fn to_wire(self) -> Self::Wire {
        match self {
            Ok(t) => WireTreeEntry {
                key: Some(t.key.to_wire()),
                data: t.data,
                parents: t.parents.to_wire(),
                children: t.children.to_wire(),
                error: None,
                tree_aux_data: t.tree_aux_data.to_wire(),
            },
            Err(e) => WireTreeEntry {
                key: e.key.to_wire(),
                error: Some(e.err.to_wire()),
                ..Default::default()
            },
        }
    }
}

impl ToApi for WireTreeEntry {
    type Api = Result<TreeEntry, SaplingRemoteApiServerError>;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(if let (key, Some(err)) = (self.key.clone(), self.error) {
            Err(SaplingRemoteApiServerError {
                key: key.to_api()?,
                err: err.to_api()?,
            })
        } else {
            Ok(TreeEntry {
                key: self
                    .key
                    .to_api()?
                    .ok_or(WireToApiConversionError::CannotPopulateRequiredField("key"))?,
                data: self.data,
                parents: self.parents.to_api()?,
                children: self.children.to_api()?,
                tree_aux_data: self.tree_aux_data.to_api()?,
            })
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WireTreeChildEntry {
    #[serde(rename = "0", default, skip_serializing_if = "is_default")]
    key: Option<WireKey>,

    #[serde(rename = "3", default, skip_serializing_if = "is_default")]
    file_metadata: Option<WireFileMetadata>,

    #[serde(rename = "4", default, skip_serializing_if = "is_default")]
    error: Option<WireSaplingRemoteApiServerError>,

    #[serde(rename = "5", default, skip_serializing_if = "is_default")]
    tree_aux_data: Option<WireTreeAuxData>,
}

impl ToWire for Result<TreeChildEntry, SaplingRemoteApiServerError> {
    type Wire = WireTreeChildEntry;

    fn to_wire(self) -> Self::Wire {
        match self {
            Ok(TreeChildEntry::File(t)) => WireTreeChildEntry {
                key: Some(t.key.to_wire()),
                file_metadata: t.file_metadata.to_wire(),
                tree_aux_data: None,
                error: None,
            },
            Ok(TreeChildEntry::Directory(t)) => WireTreeChildEntry {
                key: Some(t.key.to_wire()),
                file_metadata: None,
                tree_aux_data: t.tree_aux_data.to_wire(),
                error: None,
            },
            Err(e) => WireTreeChildEntry {
                key: e.key.to_wire(),
                error: Some(e.err.to_wire()),
                ..Default::default()
            },
        }
    }
}

impl ToApi for WireTreeChildEntry {
    type Api = Result<TreeChildEntry, SaplingRemoteApiServerError>;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(if let (key, Some(err)) = (self.key.clone(), self.error) {
            Err(SaplingRemoteApiServerError {
                key: key.to_api()?,
                err: err.to_api()?,
            })
        } else {
            Ok(
                if let (key, Some(file_metadata)) = (self.key.clone(), self.file_metadata) {
                    TreeChildEntry::File(TreeChildFileEntry {
                        key: key
                            .to_api()?
                            .ok_or(WireToApiConversionError::CannotPopulateRequiredField("key"))?,
                        file_metadata: Some(file_metadata.to_api()?),
                    })
                } else {
                    TreeChildEntry::Directory(TreeChildDirectoryEntry {
                        key: self
                            .key
                            .to_api()?
                            .ok_or(WireToApiConversionError::CannotPopulateRequiredField("key"))?,
                        tree_aux_data: self.tree_aux_data.to_api()?,
                    })
                },
            )
        })
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "for-tests"), derive(Arbitrary))]
pub struct WireTreeKeysQuery {
    pub keys: Vec<WireKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "for-tests"), derive(Arbitrary))]
pub enum WireTreeQuery {
    #[serde(rename = "1")]
    ByKeys(WireTreeKeysQuery),

    #[serde(other, rename = "0")]
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "for-tests"), derive(Arbitrary))]
pub struct WireTreeAttributesRequest {
    #[serde(rename = "0", default, skip_serializing_if = "is_default")]
    with_data: bool,

    #[serde(rename = "1", default, skip_serializing_if = "is_default")]
    with_parents: bool,

    #[serde(rename = "4", default, skip_serializing_if = "is_default")]
    with_child_metadata: bool,

    #[serde(rename = "5", default, skip_serializing_if = "is_default")]
    with_augmented_trees: bool,
}

impl ToWire for TreeAttributes {
    type Wire = WireTreeAttributesRequest;

    fn to_wire(self) -> Self::Wire {
        WireTreeAttributesRequest {
            with_data: self.manifest_blob,
            with_parents: self.parents,
            with_child_metadata: self.child_metadata,
            with_augmented_trees: self.augmented_trees,
        }
    }
}

impl ToApi for WireTreeAttributesRequest {
    type Api = TreeAttributes;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(TreeAttributes {
            child_metadata: self.with_child_metadata,
            parents: self.with_parents,
            manifest_blob: self.with_data,
            augmented_trees: self.with_augmented_trees,
        })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "for-tests"), derive(Arbitrary))]
pub struct WireTreeRequest {
    #[serde(rename = "0", default, skip_serializing_if = "is_default")]
    query: Option<WireTreeQuery>,

    #[serde(rename = "1", default, skip_serializing_if = "is_default")]
    attributes: Option<WireTreeAttributesRequest>,
}

impl ToWire for TreeRequest {
    type Wire = WireTreeRequest;

    fn to_wire(self) -> Self::Wire {
        WireTreeRequest {
            query: Some(WireTreeQuery::ByKeys(WireTreeKeysQuery {
                keys: self.keys.to_wire(),
            })),

            attributes: Some(self.attributes.to_wire()),
        }
    }
}

impl ToApi for WireTreeRequest {
    type Api = TreeRequest;
    type Error = WireToApiConversionError;

    fn to_api(self) -> Result<Self::Api, Self::Error> {
        Ok(TreeRequest {
            keys: match self.query {
                Some(WireTreeQuery::ByKeys(kq)) => kq.keys.to_api()?,
                Some(_) => {
                    return Err(WireToApiConversionError::UnrecognizedEnumVariant(
                        "WireTreeQuery",
                    ));
                }
                None => {
                    return Err(WireToApiConversionError::CannotPopulateRequiredField(
                        "keys",
                    ));
                }
            },
            attributes: self.attributes.to_api()?.unwrap_or_default(),
        })
    }
}

#[cfg(any(test, feature = "for-tests"))]
impl Arbitrary for WireTreeEntry {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let bytes: Option<Vec<u8>> = Arbitrary::arbitrary(g);
        Self {
            key: Arbitrary::arbitrary(g),
            data: bytes.map(Bytes::from),
            parents: Arbitrary::arbitrary(g),
            // Not recursing here because it causes Quickcheck to overflow the stack
            children: None,
            // TODO
            error: None,
            tree_aux_data: Arbitrary::arbitrary(g),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::tests::auto_wire_tests;

    auto_wire_tests!(
        WireTreeAttributesRequest,
        WireTreeRequest,
        WireTreeEntry,
        WireUploadTreeResponse
    );
}
