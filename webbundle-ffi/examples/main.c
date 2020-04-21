#include "webbundle-ffi.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

// Print primary url of the given bundle.
int main(int argc, char *argv[]) {
  if (argc != 2) {
    printf( "usage: %s filename", argv[0]);
    return 1;
  }
  FILE *f = fopen(argv[1], "rb");
  fseek(f, 0, SEEK_END);
  long fsize = ftell(f);
  fseek(f, 0, SEEK_SET);
  char *bytes = malloc(fsize + 1);
  size_t read_size = fread(bytes, 1, fsize, f);
  assert(read_size == fsize);

  const WebBundle* bundle = webbundle_parse(bytes, fsize);

  char primary_url[300];
  int primary_url_size = webbundle_primary_url(bundle, primary_url, 300 - 1);
  assert(primary_url_size >= 0);
  assert(primary_url_size < 300);
  primary_url[primary_url_size] = 0;

  printf("primary_url: %s\n", primary_url);

  // Closing
  fclose(f);
  free(bytes);
  webbundle_destroy((WebBundle*)(bundle));
  return 0;
}
