# File Server

This codebase explores whether the RustiCal code might be a suitable foundation for a WebDAV file server that will also extend to a web browser-based file manager

## Notes

GVFS Debugging

```
GVFS_DEBUG=1 GVFS_HTTP_DEBUG=100 /usr/lib/gvfsd --replace 2>&1 | tee gvfsd.log
```
