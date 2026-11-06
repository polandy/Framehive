// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

// The calls of libgphoto2 that take or fill one of its structs by value. Writing them
// here lets the compiler take the layout of CameraAbilities, CameraFileInfo and Camera
// from the libgphoto2 headers, instead of mirroring them in Rust (ADR 0034 §3).

#include <gphoto2/gphoto2.h>

// Makes `camera` use the driver for `model` on the port `port`, as
// gp_camera_init would after autodetection, so that it opens exactly that camera.
int framehive_gp_select(Camera *camera, const char *model, const char *port) {
  CameraAbilitiesList *abilities_list = NULL;
  GPPortInfoList *port_list = NULL;
  CameraAbilities abilities;
  GPPortInfo port_info;
  int result = gp_abilities_list_new(&abilities_list);
  if (result >= GP_OK)
    result = gp_abilities_list_load(abilities_list, NULL);
  if (result >= GP_OK)
    result = gp_abilities_list_lookup_model(abilities_list, model);
  if (result >= GP_OK)
    result = gp_abilities_list_get_abilities(abilities_list, result, &abilities);
  if (result >= GP_OK)
    result = gp_camera_set_abilities(camera, abilities);
  if (result >= GP_OK)
    result = gp_port_info_list_new(&port_list);
  if (result >= GP_OK)
    result = gp_port_info_list_load(port_list);
  if (result >= GP_OK)
    result = gp_port_info_list_lookup_path(port_list, port);
  if (result >= GP_OK)
    result = gp_port_info_list_get_info(port_list, result, &port_info);
  if (result >= GP_OK)
    result = gp_camera_set_port_info(camera, port_info);
  if (port_list != NULL)
    gp_port_info_list_free(port_list);
  if (abilities_list != NULL)
    gp_abilities_list_free(abilities_list);
  return result < GP_OK ? result : GP_OK;
}

// The size and modification time of a file. `has_size` and `has_mtime` tell whether
// the driver reports them.
int framehive_gp_file_info(Camera *camera, const char *folder, const char *file,
                           uint64_t *size, int *has_size, int64_t *mtime,
                           int *has_mtime, GPContext *context) {
  CameraFileInfo info;
  int result = gp_camera_file_get_info(camera, folder, file, &info, context);
  if (result < GP_OK)
    return result;
  *has_size = (info.file.fields & GP_FILE_INFO_SIZE) != 0;
  *size = info.file.size;
  *has_mtime = (info.file.fields & GP_FILE_INFO_MTIME) != 0;
  *mtime = (int64_t)info.file.mtime;
  return GP_OK;
}

// Reads up to `*size` bytes of the file's normal data from `offset`; `*size` is then
// the number of bytes read.
int framehive_gp_file_read(Camera *camera, const char *folder, const char *file,
                           uint64_t offset, char *buffer, uint64_t *size,
                           GPContext *context) {
  return gp_camera_file_read(camera, folder, file, GP_FILE_TYPE_NORMAL, offset,
                             buffer, size, context);
}

// Forgets the folders and files libgphoto2 has listed, so the next call asks the
// driver again.
int framehive_gp_forget_listings(Camera *camera) {
  return gp_filesystem_reset(camera->fs);
}
