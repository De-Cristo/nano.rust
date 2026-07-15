#include <XrdCl/XrdClFile.hh>

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <string>

namespace {

struct NanoRootioXrootdFile {
  XrdCl::File file;
};

void copy_error(char *buffer, std::size_t capacity, const std::string &message) {
  if (buffer == nullptr || capacity == 0) {
    return;
  }
  const std::size_t length = message.size() < capacity - 1 ? message.size() : capacity - 1;
  std::memcpy(buffer, message.data(), length);
  buffer[length] = '\0';
}

}  // namespace

extern "C" void *nano_rootio_xrootd_open(const char *url, char *error, std::size_t error_capacity) {
  if (url == nullptr) {
    copy_error(error, error_capacity, "missing XRootD URL");
    return nullptr;
  }
  try {
    auto *handle = new NanoRootioXrootdFile();
    const XrdCl::XRootDStatus status = handle->file.Open(url, XrdCl::OpenFlags::Read);
    if (!status.IsOK()) {
      copy_error(error, error_capacity, status.ToString());
      delete handle;
      return nullptr;
    }
    return handle;
  } catch (const std::exception &exception) {
    copy_error(error, error_capacity, exception.what());
  } catch (...) {
    copy_error(error, error_capacity, "unknown XRootD open failure");
  }
  return nullptr;
}

extern "C" int nano_rootio_xrootd_read(
    void *raw_handle,
    std::uint64_t offset,
    std::uint32_t size,
    void *buffer,
    std::uint32_t *bytes_read,
    char *error,
    std::size_t error_capacity) {
  if (raw_handle == nullptr || buffer == nullptr || bytes_read == nullptr) {
    copy_error(error, error_capacity, "invalid XRootD read arguments");
    return 1;
  }
  try {
    auto *handle = static_cast<NanoRootioXrootdFile *>(raw_handle);
    std::uint32_t actual = 0;
    const XrdCl::XRootDStatus status = handle->file.Read(offset, size, buffer, actual);
    if (!status.IsOK()) {
      copy_error(error, error_capacity, status.ToString());
      return 1;
    }
    *bytes_read = actual;
    return 0;
  } catch (const std::exception &exception) {
    copy_error(error, error_capacity, exception.what());
  } catch (...) {
    copy_error(error, error_capacity, "unknown XRootD read failure");
  }
  return 1;
}

extern "C" void nano_rootio_xrootd_close(void *raw_handle) {
  if (raw_handle == nullptr) {
    return;
  }
  auto *handle = static_cast<NanoRootioXrootdFile *>(raw_handle);
  handle->file.Close();
  delete handle;
}
