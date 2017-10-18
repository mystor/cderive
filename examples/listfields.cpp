#include <stdio.h>
#include "derive.h"

#define DECL_LISTFIELDS \
  public: \
    static int field_count(); \
    static const char** field_names();

class Example {
  DECL_LISTFIELDS
public:
  int first;
  bool second;
  char third;
};

DERIVE(Example, ListFields);

int main() {
  int count = Example::field_count();
  const char** names = Example::field_names();
  for (int i = 0; i < count; ++i) {
    printf("%s\n", names[i]);
  }
}
