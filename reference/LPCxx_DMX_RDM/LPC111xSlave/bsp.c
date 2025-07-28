/*****************************************************************************/
/*  Copyright (c) 2011 NXP B.V.  All rights are reserved.                    */
/*  Reproduction in whole or in part is prohibited without the prior         */
/*  written consent of the copyright owner.                                  */
/*                                                                           */
/*  This software and any compilation or derivative thereof is, and          */
/*  shall remain the proprietary information of NXP and is                   */
/*  highly confidential in nature. Any and all use hereof is restricted      */
/*  and is subject to the terms and conditions set forth in the              */
/*  software license agreement concluded with NXP B.V.                       */
/*                                                                           */
/*  Under no circumstances is this software or any derivative thereof        */
/*  to be combined with any Open Source Software, exposed to, or in any      */
/*  way licensed under any Open License Terms without the express prior      */
/*  written permission of the copyright owner.                               */
/*                                                                           */
/*  For the purpose of the above, the term Open Source Software means        */
/*  any software that is licensed under Open License Terms. Open             */
/*  License Terms means terms in any license that require as a               */
/*  condition of use, modification and/or distribution of a work             */
/*                                                                           */
/*  1. the making available of source code or other materials                */
/*     preferred for modification, or                                        */
/*                                                                           */
/*  2. the granting of permission for creating derivative                    */
/*     works, or                                                             */
/*                                                                           */
/*  3. the reproduction of certain notices or license terms                  */
/*     in derivative works or accompanying documentation, or                 */
/*                                                                           */
/*  4. the granting of a royalty-free license to any party                   */
/*     under Intellectual Property Rights                                    */
/*                                                                           */
/*  regarding the work and/or any work that contains, is combined with,      */
/*  requires or otherwise is based on the work.                              */
/*                                                                           */
/*  This software is provided for ease of recompilation only.                */
/*  Modification and reverse engineering of this software are strictly       */
/*  prohibited.                                                              */
/*                                                                           */
/*****************************************************************************/

/*******************************************************************
 * standard include files
 *******************************************************************/

#include <stdbool.h>

/*******************************************************************
 * project specific include files
 *******************************************************************/

#include "bsp.h"
#include "uart.h"
#include "wdt.h"
#include "i2c.h"
#include "lcd.h"
#include "timer_pwm.h"

#define SYS_TICK_RATE_HZ       1000
#define NR_OF_MSEC_COUNTERS    4

#define IAP_LOCATION           0x1FFF1FF1
typedef void (*IAP)(unsigned int *pcommand, unsigned int *presult);

static          uint8_t  led_state;
static volatile uint32_t sys_seconds_cnt;
static volatile uint16_t sys_millisec_cnt;
static          uint32_t app_millisec_cnt[NR_OF_MSEC_COUNTERS];
static          bool     app_millisec_cnt_avail[NR_OF_MSEC_COUNTERS];

/******************************************************************************
 * global functions
 *****************************************************************************/

void TickHandler(void)
{
    uint32_t i;
    uint32_t millisec_increment = 1000 / SYS_TICK_RATE_HZ;
    
    sys_millisec_cnt += millisec_increment;
    if (sys_millisec_cnt >= 1000)
    {
        sys_millisec_cnt -= 1000;
        sys_seconds_cnt++;
        led_state ^= 1;
#ifndef PROTOTYPE_BOARD
        if (led_state) {
            LPC_GPIO0->DATA |=  (1 << 2); /* Led on P0.2 off */
        } else {
            LPC_GPIO0->DATA &= ~(1 << 2); /* Led on P0.2 on */
        }
#else
        if (led_state) {
            LPC_GPIO0->DATA |=  (1 << 7); /* Led on P0.7 */
        } else {
            LPC_GPIO0->DATA &= ~(1 << 7); /* Led on P0.7 */
        }
#endif
    }
    for (i=0; i<NR_OF_MSEC_COUNTERS; i++)
    {
        app_millisec_cnt[i] += millisec_increment;
    }
}

void bsp_init(void)
{
    uint32_t i;
    
    /* Init global variables */
    led_state = 0;
    sys_seconds_cnt = 0;
    sys_millisec_cnt = 0;
    for (i=0; i<NR_OF_MSEC_COUNTERS; i++)
    {
        app_millisec_cnt_avail[i] = true;
    }

    /* Setup the system tick timer */
    NVIC_SetPriority(SysTick_IRQn, SYSTICK_IRQ_PRIORITY);
    SysTick_Config(LPC_CORE_CLOCKSPEED_HZ / SYS_TICK_RATE_HZ);

#ifdef SUPPORT_I2C_BUS
    LPC_SYSCON->PRESETCTRL |= (1 << 1);
#endif
    
    LPC_SYSCON->SYSAHBCLKCTRL |= (1 <<  6) |  /* Enable AHB clock to the GPIO domain */
#ifdef SUPPORT_I2C_BUS
                                 (1 <<  5) |  /* Enable AHB clock to the I2C */
#endif
                                 (1 <<  7) |  /* Enable AHB clock to the 16-bit counter/timer 0 */
                                 (1 <<  8) |  /* Enable AHB clock to the 16-bit counter/timer 1 */
                                 (1 <<  9) |  /* Enable AHB clock to the 32-bit counter/timer 0 */
                                 (1 << 12) |  /* Enable AHB clock to the UART */
                                 (1 << 15);   /* Enable AHB clock to the WDT */
    
    /* Configuring LED pin P0.3 (traffic LED) */
    LPC_IOCON->PIO0_3 = 0xC0;     // select PIO mode, no pull up/down, disable hysteresis
    LPC_GPIO0->DIR |= (1 << 3);   // set PIO pin P0.3 as output
#ifndef PROTOTYPE_BOARD
    /* Configuring LED pin P0.2 (heartbeat LED) */
    LPC_IOCON->PIO0_2 = 0xC0;     // select PIO mode, no pull up/down, disable hysteresis
    LPC_GPIO0->DIR |= (1 << 2);   // set PIO pin P0.2 as output
    /* Configure pin P0.7 for DMX bus data direction control */
    LPC_IOCON->PIO0_7 = 0xC0;     // select PIO mode, no pull up/down, disable hysteresis
    LPC_GPIO0->DIR |= (1 << 7);   // set PIO pin P0.7 as output
#else
    /* Configuring LED pin P0.7 */
    LPC_IOCON->PIO0_7 = 0xC0;     // select PIO mode, no pull up/down, disable hysteresis
    LPC_GPIO0->DIR |= (1 << 7);   // set PIO pin P0.7 as output
    /* Configure pin P2.6 for DMX bus data direction control */
    LPC_IOCON->PIO2_6 = 0xC0;     // select PIO mode, no pull up/down, disable hysteresis
    LPC_GPIO2->DIR |= (1 << 6);   // set PIO pin P2.6 as output
#endif
    bsp_recv_from_dmx_bus();
    /* Configuring the DIP switches P2.8, P2.9, P2.10 and P2.11 */
    LPC_IOCON->PIO2_8 = 0xD0;     // select PIO mode, pull up, disable hysteresis
    LPC_GPIO2->DIR &= ~(1 << 8);  // set PIO pin P2.8 as input
    LPC_IOCON->PIO2_9 = 0xD0;     // select PIO mode, pull up, disable hysteresis
    LPC_GPIO2->DIR &= ~(1 << 9);  // set PIO pin P2.9 as input
    LPC_IOCON->PIO2_10 = 0xD0;    // select PIO mode, pull up, disable hysteresis
    LPC_GPIO2->DIR &= ~(1 << 10); // set PIO pin P2.10 as input
    LPC_IOCON->PIO2_11 = 0xD0;    // select PIO mode, pull up, disable hysteresis
    LPC_GPIO2->DIR &= ~(1 << 11); // set PIO pin P2.11 as input
    
    /* Configure CT16B0 and CT16B1 match pins 0 and 1 */
    LPC_IOCON->PIO0_8  = 0xC2;    // select CT16B0_MAT0
    LPC_IOCON->PIO0_9  = 0xC2;    // select CT16B0_MAT1
    Timer_PWM_Init(CT16B0);
    LPC_IOCON->PIO1_9  = 0xC1;    // select CT16B1_MAT0
    LPC_IOCON->PIO1_10 = 0xC2;    // select CT16B1_MAT1
    Timer_PWM_Init(CT16B1);
    
    /* Configuring UART RXD pin */
    LPC_IOCON->PIO1_6 = 0xD1;     // select RXD mode, pull up, disable hysteresis
    /* Configuring UART TXD pin */
    LPC_IOCON->PIO1_7 = 0xD1;     // select TXD mode, pull up, disable hysteresis
    Uart_Init();

#ifdef SUPPORT_I2C_BUS
    /* Configuring I2C SCL pin */
    LPC_IOCON->PIO0_4 = 0x0001;   // select SCL mode, standard mode
    /* Configuring I2C SDA pin */
    LPC_IOCON->PIO0_5 = 0x0001;   // select SDA mode, standard mode
    I2C_Init(I2CMASTER,0,0,0);
    lcd_init();
#endif

#ifdef SUPPORT_MANUAL_CONTROL
    /* Configuring joystick pins P3.0, P3.1, P3.2, P3.3 and P3.4 */
    LPC_IOCON->PIO3_0 = 0x00;     // select PIO mode, disable hysteresis
    LPC_IOCON->PIO3_1 = 0x00;     // select PIO mode, disable hysteresis
    LPC_IOCON->PIO3_2 = 0x00;     // select PIO mode, disable hysteresis
    LPC_IOCON->PIO3_3 = 0x00;     // select PIO mode, disable hysteresis
    LPC_IOCON->PIO3_4 = 0x00;     // select PIO mode, disable hysteresis
    LPC_GPIO3->DIR   &= ~0x1F;    // set PIO pins P3.0, P3.1, P3.2, P3.3 and P3.4 as input
#endif

    WDT_Init(6);                  // 6 seconds timeout
}

void bsp_control_led(uint8_t led_nr, uint8_t led_state)
{
    if (led_state) {
        LPC_GPIO0->DATA |=  (1 << 3); /* Led on P0.3 off */
    } else {
        LPC_GPIO0->DATA &= ~(1 << 3); /* Led on P0.3 on */
    }
}

//#define DMX_SLAVE_FIRST_VERSION

void bsp_recv_from_dmx_bus(void)
{
#ifndef PROTOTYPE_BOARD
  #ifdef DMX_SLAVE_FIRST_VERSION
    LPC_GPIO0->DATA &= ~(1 << 7); // drive PIO pin P0.7 low (recv from DMX bus)
  #else
    LPC_GPIO0->DATA |= (1 << 7); // drive PIO pin P0.7 high (recv from DMX bus)
  #endif
#else
    LPC_GPIO2->DATA &= ~(1 << 6); // drive PIO pin P2.6 low (recv from DMX bus)
#endif
}

void bsp_xmit_to_dmx_bus(void)
{
#ifndef PROTOTYPE_BOARD
  #ifdef DMX_SLAVE_FIRST_VERSION
    LPC_GPIO0->DATA |= (1 << 7); // drive PIO pin P0.7 high (xmit to DMX bus)
  #else
    LPC_GPIO0->DATA &= ~(1 << 7); // drive PIO pin P0.7 low (xmit to DMX bus)
  #endif
#else
    LPC_GPIO2->DATA |= (1 << 6); // drive PIO pin P2.6 high (xmit to DMX bus)
#endif
}

uint8_t bsp_claim_msec_cnt(void)
{
    uint8_t counterId = 0;
    
    while(counterId < NR_OF_MSEC_COUNTERS)
    {
        if (app_millisec_cnt_avail[counterId]) {
            app_millisec_cnt[counterId] = 0;
            app_millisec_cnt_avail[counterId] = false;
            break;
        }
        counterId++;
    }
    return counterId;
}

void bsp_reset_msec_cnt(uint8_t counterId)
{
    if (counterId < NR_OF_MSEC_COUNTERS) {
        app_millisec_cnt[counterId] = 0;
    }
}

uint32_t bsp_get_msec_cnt(uint8_t counterId)
{
    if (counterId < NR_OF_MSEC_COUNTERS) {
        return app_millisec_cnt[counterId];
    }
    return 0;
}

void bsp_get_sys_uptime(uint32_t *psec_cnt, uint16_t *pmsec_cnt)
{
    // we do not want to disable the systick interrupt, so we have to deal
    // with a systick during the execution of this function
    uint32_t sec_cnt_1  = sys_seconds_cnt;
    uint16_t msec_cnt_1 = sys_millisec_cnt;
    uint32_t sec_cnt_2  = sys_seconds_cnt;
    uint16_t msec_cnt_2 = sys_millisec_cnt;
    if ((sec_cnt_1 == sec_cnt_2) && (msec_cnt_2 < msec_cnt_1))
    {
        sec_cnt_2 += 1;
    }
    *psec_cnt  = sec_cnt_2;
    *pmsec_cnt = msec_cnt_2;
}

void bsp_delay_usec(uint32_t usecDelay)
{
    LPC_TMR32B0->TCR = 0x02;          /* disable and reset this CounterTimer */
    LPC_TMR32B0->PR  = 0x00;          /* set prescaler to zero */
    LPC_TMR32B0->MR0 = usecDelay * (SystemAHBFrequency / 1000000);
    LPC_TMR32B0->IR  = 0x1F;          /* reset all interrupt flags */
    LPC_TMR32B0->MCR = 0x04;          /* stop timer on match */
    LPC_TMR32B0->TCR = 0x01;          /* CounterTimer enable */
    while (LPC_TMR32B0->TCR & 0x01);  /* wait until delay time has elapsed */
}

void bsp_delay_msec(uint32_t msecDelay)
{
    uint32_t i;
    for (i=0; i<msecDelay; i++)
    {
        bsp_delay_usec(1000); // wait 1 msec
    }
}

uint8_t bsp_read_dip_switches(void)
{
    return ((LPC_GPIO2->DATA >> 8) & 0x0F);
}

uint32_t bsp_get_id(void)
{
    IAP       iap_entry  = (IAP)IAP_LOCATION;
    uint32_t  command[5] = {58,0,0,0,0}; // Read device serial number
    uint32_t  result[5]  = { 0,0,0,0,0};

    iap_entry(command, result);
    if (result[0] != 0) return 0;        // not succesful
    return (((result[1] ^ result[2]) ^ result[3]) ^ result[4]);
}

/* End of file */
