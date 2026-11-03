// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

// Setters for decode parameters that the LibRaw C API leaves out. Writing the fields
// here lets the compiler take their offsets from the LibRaw header, instead of
// mirroring libraw_data_t in Rust (ADR 0028 §5).

#include <libraw/libraw.h>

void framehive_libraw_set_use_camera_wb(libraw_data_t *lr, int value) {
  lr->params.use_camera_wb = value;
}

void framehive_libraw_set_user_flip(libraw_data_t *lr, int value) {
  lr->params.user_flip = value;
}
