# 5. Extension points

Part of the [architecture](README.md).

| Extension point | Trait | Examples |
|---|---|---|
| Device source | `devices::DeviceSource` | Mount, gphoto2 (PTP), MTP, iOS |
| Metadata format | `metadata::MetadataCodec` | XMP sidecar, embedded XMP, EXIF, IPTC-IIM |
| Preview decoder | `previews::PreviewDecoder` | Embedded JPEG, RAW (LibRaw), HEIF, video (ffmpeg), Immich thumbnails |
| Job kind | `jobs::JobKind` | Import, writing XMP, hashing, faces |
| Search provider | `catalog::SearchProvider` | Full text, filters, Immich smart search, local CLIP similarity |
| Face provider | `catalog::FaceProvider` | Immich faces and people (ADR 0013), later local recognition (ONNX) |
| Export target | `export::Exporter` | JPEG/AVIF, folder, hook |
| External program | `kernel::ExternalApp` | darktable, RawTherapee, GIMP |
| Migration source | `migrate::Source` | digiKam, XMP folder, later Lightroom |

The face provider lives in a service, not in the people module, because modules never
depend on each other: the immich module provides faces, the people module consumes them.

For every extension point there is a **conformance test suite** (contract tests) that
every implementation must pass, e.g. `devices::testing::check_device_source`.
