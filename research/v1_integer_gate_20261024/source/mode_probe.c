/* Isolated child only: all inputs are hexadecimal integer bit patterns.
 * Restore MXCSR before any logging. No host/user-process setting is changed. */
#include <stdint.h>
#include <stdio.h>
#include <inttypes.h>
#include <string.h>
#include <xmmintrin.h>
extern int gate_bits(const uint64_t*,const uint64_t*,uint64_t*);
int main(void){uint64_t e[30],s[33],base[6],got[6];unsigned n=0,checks=0;
 unsigned saved=_mm_getcsr();
 while(scanf("%"SCNx64,&e[0])==1){
  for(int i=1;i<30;i++)if(scanf("%"SCNx64,&e[i])!=1)return 2;
  for(int i=0;i<33;i++)if(scanf("%"SCNx64,&s[i])!=1)return 2;
  int expected=gate_bits(e,s,base);
  for(unsigned rc=0;rc<4;rc++)for(unsigned ftz=0;ftz<2;ftz++)for(unsigned daz=0;daz<2;daz++){
   unsigned config=(saved&~0xe040u)|(rc<<13)|(ftz<<15)|(daz<<6);
   _mm_setcsr(config);unsigned active=_mm_getcsr();int code=gate_bits(e,s,got);unsigned after=_mm_getcsr();_mm_setcsr(saved);
   if(active!=config||after!=config||code!=expected||memcmp(base,got,sizeof base)){printf("FAIL case=%u config=%x\n",n,config);return 1;}checks++;
  }n++;
 }
 if(_mm_getcsr()!=saved)return 3;
 printf("PASS_INTEGER_ONLY_MXCSR cases=%u modes=16 checks=%u initial=%x restored=%x\n",n,checks,saved,_mm_getcsr());return 0;
}
