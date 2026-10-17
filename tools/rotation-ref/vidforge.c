// [M] brute-force a SECOND PREIMAGE for a 4-byte vid: find nonce so SHA256(msg_prefix || nonce_hex12)[0..4) == target.
// Usage: vidforge <target_hex8> <msg_prefix_hex> <threads>.  The prefix must end right before the 12-hex-digit nonce
// and the nonce must be the LAST 12 bytes of the message. Midstate reuse: one compression per trial when tail<56 B.
#include <openssl/sha.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdatomic.h>
static unsigned char target[4], *pre; static size_t prelen; static int T;
static atomic_int found; static atomic_ullong tries; static unsigned long long hit;
static SHA256_CTX base; static size_t tailfix;
static void *work(void *arg){
  long id=(long)arg; unsigned char tail[128]; size_t fixed=prelen-64;
  memcpy(tail,pre+64,fixed); static const char hx[]="0123456789abcdef";
  unsigned long long n=id, local=0; unsigned char out[32];
  while(!atomic_load_explicit(&found,memory_order_relaxed)){
    for(int k=0;k<4096;k++){
      unsigned long long v=n; for(int d=11;d>=0;d--){ tail[fixed+d]=hx[v&15]; v>>=4; }
      SHA256_CTX c=base; SHA256_Update(&c,tail,fixed+12); SHA256_Final(out,&c);
      if(out[0]==target[0]&&out[1]==target[1]&&out[2]==target[2]&&out[3]==target[3]){ hit=n; atomic_store(&found,1); break; }
      n+=T; local++;
    }
    atomic_fetch_add(&tries,local); local=0;
  }
  return 0;
}
int main(int argc,char**argv){
  if(argc<4) return 2; T=atoi(argv[3]);
  for(int i=0;i<4;i++){ unsigned x; sscanf(argv[1]+2*i,"%2x",&x); target[i]=x; }
  prelen=strlen(argv[2])/2; pre=malloc(prelen); for(size_t i=0;i<prelen;i++){ unsigned x; sscanf(argv[2]+2*i,"%2x",&x); pre[i]=x; }
  if(prelen<64){ fprintf(stderr,"prefix must be >=64 bytes\n"); return 3; }
  if(prelen-64+12>=56){ fprintf(stderr,"tail too long\n"); return 4; }
  SHA256_Init(&base); SHA256_Update(&base,pre,64);
  struct timespec a,b; clock_gettime(CLOCK_MONOTONIC,&a);
  pthread_t th[64]; for(long i=0;i<T;i++) pthread_create(&th[i],0,work,(void*)i);
  for(int i=0;i<T;i++) pthread_join(th[i],0);
  clock_gettime(CLOCK_MONOTONIC,&b);
  double s=(b.tv_sec-a.tv_sec)+(b.tv_nsec-a.tv_nsec)/1e9;
  printf("{\"nonce_counter\": %llu, \"tries_approx\": %llu, \"seconds\": %.1f, \"threads\": %d}\n",hit,(unsigned long long)atomic_load(&tries),s,T);
  return 0;
}
