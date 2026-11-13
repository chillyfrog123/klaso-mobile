#ifndef LIBRUS_MOBILE_CORE_H
#define LIBRUS_MOBILE_CORE_H
#ifdef __cplusplus
extern "C" {
#endif
/* UTF-8 JSON, NUL-terminated. Run off the UI thread. Do not log arguments/results. */
char *librus_mobile_request(const char *input);
void librus_mobile_free(char *response);
#ifdef __cplusplus
}
#endif
#endif
