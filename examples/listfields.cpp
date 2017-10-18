#include "derive.h"

#define DECL_LISTFIELDS \
  public: static int field_count();

class Example {
  DECL_LISTFIELDS
public:
  int first;
  bool second;
  char third;
};

DERIVE(Example, ListFields);
