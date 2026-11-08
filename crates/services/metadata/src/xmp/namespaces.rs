// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Namespace URIs and the preferred prefixes of the known ones (ADR 0039, decision 3).

/// The namespace of `x:xmpmeta`.
pub const ADOBE_META: &str = "adobe:ns:meta/";
/// The RDF namespace.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// The XML namespace of `xml:lang`, bound without a declaration.
pub const XML: &str = "http://www.w3.org/XML/1998/namespace";
/// Dublin Core, prefix `dc`.
pub const DUBLIN_CORE: &str = "http://purl.org/dc/elements/1.1/";
/// XMP Basic, prefix `xmp`.
pub const XMP_BASIC: &str = "http://ns.adobe.com/xap/1.0/";
/// Photoshop, prefix `photoshop`.
pub const PHOTOSHOP: &str = "http://ns.adobe.com/photoshop/1.0/";
/// Lightroom, prefix `lr`.
pub const LIGHTROOM: &str = "http://ns.adobe.com/lightroom/1.0/";
/// MWG keywords, prefix `mwg-kw`.
pub const MWG_KEYWORDS: &str = "http://www.metadataworkinggroup.com/schemas/keywords/";
/// Framehive's own namespace, prefix `framehive` (`docs/xmp-namespace.md`).
pub const FRAMEHIVE: &str = "https://polandy.github.io/framehive/ns/1.0/";

/// The prefixes of the packet structure itself; no property namespace may take them.
pub(crate) const STRUCTURE_PREFIXES: [&str; 2] = ["x", "rdf"];

/// Known namespaces and their preferred prefixes. A file's prefix for one of them is
/// replaced, and no other namespace gets one of these prefixes.
pub(crate) const PREFERRED_PREFIXES: [(&str, &str); 29] = [
    (DUBLIN_CORE, "dc"),
    (XMP_BASIC, "xmp"),
    ("http://ns.adobe.com/xap/1.0/rights/", "xmpRights"),
    ("http://ns.adobe.com/xap/1.0/mm/", "xmpMM"),
    ("http://ns.adobe.com/xap/1.0/bj/", "xmpBJ"),
    ("http://ns.adobe.com/xap/1.0/t/pg/", "xmpTPg"),
    ("http://ns.adobe.com/xmp/1.0/DynamicMedia/", "xmpDM"),
    ("http://ns.adobe.com/xmp/Identifier/qual/1.0/", "xmpidq"),
    ("http://ns.adobe.com/xap/1.0/sType/ResourceEvent#", "stEvt"),
    ("http://ns.adobe.com/xap/1.0/sType/ResourceRef#", "stRef"),
    ("http://ns.adobe.com/xap/1.0/sType/Dimensions#", "stDim"),
    ("http://ns.adobe.com/xmp/sType/Area#", "stArea"),
    (PHOTOSHOP, "photoshop"),
    ("http://ns.adobe.com/tiff/1.0/", "tiff"),
    ("http://ns.adobe.com/exif/1.0/", "exif"),
    ("http://cipa.jp/exif/1.0/", "exifEX"),
    ("http://ns.adobe.com/exif/1.0/aux/", "aux"),
    ("http://ns.adobe.com/camera-raw-settings/1.0/", "crs"),
    (LIGHTROOM, "lr"),
    (
        "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/",
        "Iptc4xmpCore",
    ),
    ("http://iptc.org/std/Iptc4xmpExt/2008-02-29/", "Iptc4xmpExt"),
    ("http://ns.useplus.org/ldf/xmp/1.0/", "plus"),
    (
        "http://www.metadataworkinggroup.com/schemas/regions/",
        "mwg-rs",
    ),
    (MWG_KEYWORDS, "mwg-kw"),
    (
        "http://www.metadataworkinggroup.com/schemas/collections/",
        "mwg-coll",
    ),
    ("http://darktable.sf.net/", "darktable"),
    ("http://www.digikam.org/ns/1.0/", "digiKam"),
    ("http://ns.microsoft.com/photo/1.0/", "MicrosoftPhoto"),
    (FRAMEHIVE, "framehive"),
];

/// The preferred prefix of a known namespace.
pub(crate) fn preferred_prefix(namespace: &str) -> Option<&'static str> {
    PREFERRED_PREFIXES
        .iter()
        .find(|(uri, _)| *uri == namespace)
        .map(|(_, prefix)| *prefix)
}
