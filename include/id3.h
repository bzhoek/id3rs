#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define ID3HEADER_SIZE 10

typedef struct ID3rs ID3rs;

struct ID3rs *id3_read(const char *file);

void id3_write(struct ID3rs *ptr, const char *file);

void id3_set_popularity(struct ID3rs *ptr, const char *email, uint8_t rating);

void id3_free(struct ID3rs *ptr);
