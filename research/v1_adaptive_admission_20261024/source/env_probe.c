#include <xmmintrin.h>
#include <stdio.h>
extern int environment_code(void);
int main(void){unsigned original=_mm_getcsr(),good=0,bad=0;
for(unsigned rc=0;rc<4;rc++)for(unsigned ftz=0;ftz<2;ftz++)for(unsigned daz=0;daz<2;daz++){
unsigned config=(original&~0xe040u)|(rc<<13)|(ftz<<15)|(daz<<6);_mm_setcsr(config);int code=environment_code();unsigned after=_mm_getcsr();_mm_setcsr(original);
if(after!=config || code!=(rc||ftz||daz)){printf("FAIL %x %d\n",config,code);return 1;}if(code)bad++;else good++;}
printf("PASS_ENVIRONMENT_GUARD accepted=%u refused=%u original=%x restored=%x\n",good,bad,original,_mm_getcsr());return _mm_getcsr()!=original;
}
