#include "derive.h"

#define NON_CC_REFCNT __attribute__((annotate("non_cc_refcnt")));

class nsISupports {};
class NON_CC_REFCNT DefaultRefCnt {};
class CycleCollectingRefCnt {};
class nsXPCOMCycleCollectionParticipant {
  virtual void Unlink();
};

template<typename T>
class RefPtr {};

class Bleargh {

};

class Foo : public nsISupports {
  // NOTE: This isn't how it works, and we know it.
  class cycleCollection : nsXPCOMCycleCollectionParticipant {
    virtual void Unlink() override;
  };
  static cycleCollection _cycleCollectionGlobal;
private:
  nsCCRefCnt mRefCnt;

  RefPtr<nsISupports> mGetMe;
  RefPtr<Bleargh> mNotMe;

};

DERIVE(Foo, CycleCollection)
