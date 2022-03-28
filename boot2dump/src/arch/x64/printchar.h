#ifndef __PRINTCHAR_H__
#define __PRINTCHAR_H__

#include <types.h>

#define COLOR         0x03
#define TAB_SIZE      4
#define SCREEN_HEIGHT 25
#define SCREEN_WIDTH  80

#define IOPORT_SERIAL 0x3f8
#define RBR           0
#define DLL           0
#define DLH           1
#define IER           1
#define FCR           2
#define LCR           3
#define LSR           5
#define TX_READY      0x20

void printchar_init(void);

#endif
