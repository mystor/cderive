#ifndef __derive_h_
#define __derive_h_

#define DERIVE(clazz, ...) \
  __attribute__((annotate("DERIVE=" #__VA_ARGS__))) \
  typedef clazz __ ## clazz ## _derive_marker;

#endif // defined(__derive_h_)
