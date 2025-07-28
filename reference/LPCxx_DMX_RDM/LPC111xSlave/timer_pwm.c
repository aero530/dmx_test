/*******************************************************************************
*   timer_pwm.c:  PWM module file for NXP LPC11xx Family Microprocessors
*
*   Copyright(C) 2011, NXP B.V.
*   All rights reserved.
*
*******************************************************************************/

#include "bsp.h"
#include "timer_pwm.h"

/*******************************************************************************
*                                       GLOBAL FUNCTIONS
*******************************************************************************/

/*******************************************************************************
* Function name: Timer_PWM_Init()
* Description  : Initialises the requested CounterTimer as PWM pheripheral
* 				 Sets the default PWM frequency. Duty set to 50%
* Argument(s)  : ctId - counter timer ID
* Return(s)    : none.
*******************************************************************************/

void Timer_PWM_Init(uint8_t ctId)
{
    uint32_t pclk_freq = SystemAHBFrequency;/* get the clock of the timer unit */
    uint32_t pwmfrq = pclk_freq / PWM_FREQ;

    switch (ctId)
    {
    case CT16B0:
        LPC_TMR16B0->TCR  = 0x02;     /* disable and reset this CounterTimer */
        LPC_TMR16B0->PR   = 0x00;     /* set prescaler to zero */
        LPC_TMR16B0->MR3  = pwmfrq;   /* set the default frequency in MR3 */
        Timer_PWM_Set_Duty_Cycle(ctId, CTMAT0, PWM_DUTY);
        Timer_PWM_Set_Duty_Cycle(ctId, CTMAT1, PWM_DUTY);
        LPC_TMR16B0->MCR  = 0x400;    /* reset timer on match of MR3, no interrupts */
        LPC_TMR16B0->PWMC = 0x03;     /* select PWM mode for CT16B0_MAT0 and CT16B0_MAT1 */
        LPC_TMR16B0->IR   = 0x1F;     /* reset all interrupt flags */
        LPC_TMR16B0->CTCR = 0x00;     /* use the CounterTimer in timer mode */
        LPC_TMR16B0->TCR  = 0x01;     /* CounterTimer enable */
        break;
    case CT16B1:
        LPC_TMR16B1->TCR  = 0x02;     /* disable and reset this CounterTimer */
        LPC_TMR16B1->PR   = 0x00;     /* set prescaler to zero */
        LPC_TMR16B1->MR3  = pwmfrq;   /* set the default frequency in MR3 */
        Timer_PWM_Set_Duty_Cycle(ctId, CTMAT0, PWM_DUTY);
        Timer_PWM_Set_Duty_Cycle(ctId, CTMAT1, PWM_DUTY);
        LPC_TMR16B1->MCR  = 0x400;    /* reset timer on match of MR3, no interrupts */
        LPC_TMR16B1->PWMC = 0x03;     /* select PWM mode for CT16B1_MAT0 and CT16B1_MAT1 */
        LPC_TMR16B1->IR   = 0x1F;     /* reset all interrupt flags */
        LPC_TMR16B1->CTCR = 0x00;     /* use the CounterTimer in timer mode */
        LPC_TMR16B1->TCR  = 0x01;     /* CounterTimer enable */
        break;
    default:
        break;
    }
}

/*******************************************************************************
* Function name: Timer_PWM_Set_Frequency()
* Description  : Sets a new PWM frequency on requested CounterTimer and 
*                keeps the duty cycle intact
* Argument(s)  : ctId     - counter timer ID
*                new_freq - new frequency of PWM signal 
* Return(s)    : actual set frequency of PWM,
*                this might be different due to rounding effects
*******************************************************************************/

uint32_t Timer_PWM_Set_Frequency(uint8_t ctId, uint16_t new_freq)
{
    uint32_t pclk_freq = SystemAHBFrequency;/* get the clock of the timer unit */
    uint32_t pwmfrq = pclk_freq / new_freq;
    uint32_t match0;
    uint32_t match1;
    float    adaptation;

    switch (ctId)
    {
    case CT16B0:
        /* adapt MR0 and MR1 to keep the same duty cycle */
        adaptation = (float)pwmfrq / (float)LPC_TMR16B0->MR3; 
        match0 = (uint32_t) ((float)LPC_TMR16B0->MR0 * adaptation);	
        match1 = (uint32_t) ((float)LPC_TMR16B0->MR1 * adaptation);	
        LPC_TMR16B0->MR3 = pwmfrq;   /* set the new frequency in MR3 */
        LPC_TMR16B0->MR0 = match0;   /* reprogram the duty cycle in MR0 */
        LPC_TMR16B0->MR1 = match1;   /* reprogram the duty cycle in MR1 */
        LPC_TMR16B0->TCR = 0x02;     /* disable and reset this CounterTimer */
        LPC_TMR16B0->TCR = 0x01;     /* CounterTimer enable */
        break;
    case CT16B1:
        /* adapt MR0 and MR1 to keep the same duty cycle */
        adaptation = (float)pwmfrq / (float)LPC_TMR16B1->MR3; 
        match0 = (uint32_t) ((float)LPC_TMR16B1->MR0 * adaptation);	
        match1 = (uint32_t) ((float)LPC_TMR16B1->MR1 * adaptation);	
        LPC_TMR16B1->MR3 = pwmfrq;   /* set the new frequency in MR3 */
        LPC_TMR16B1->MR0 = match0;   /* reprogram the duty cycle in MR0 */
        LPC_TMR16B1->MR1 = match1;   /* reprogram the duty cycle in MR1 */
        LPC_TMR16B1->TCR = 0x02;     /* disable and reset this CounterTimer */
        LPC_TMR16B1->TCR = 0x01;     /* CounterTimer enable */
        break;
    default:
        break;
    }
    return (pclk_freq / pwmfrq);
}

/*******************************************************************************
* Function name: Timer_PWM_Set_Duty_Cycle()
* Description  : Sets the duty cycle on requested CounterTimer
* Argument(s)  : ctId  - counter timer ID
*                matId - match output ID
*                duty  - duty cycle, 1 - 100 %
* Return(s)    : none.
*******************************************************************************/

void Timer_PWM_Set_Duty_Cycle(uint8_t ctId, uint8_t matId, uint8_t duty)
{
    uint32_t match;
    uint32_t percentage;

    /* convert duty value (0-255) into a percentage (0-100%) */
    percentage = (((uint32_t)duty * 4) + 5) / 10;
    if (percentage > 100) percentage = 100;
    switch (ctId)
    {
    case CT16B0:
        match = (uint32_t) ((float)LPC_TMR16B0->MR3 * ((float)0.01 * percentage));
        if (matId == CTMAT0)
        {
            LPC_TMR16B0->MR0 = match;    /* adapt MR0 */   
        }
        else if (matId == CTMAT1)
        {
            LPC_TMR16B0->MR1 = match;    /* adapt MR1 */   
        }
        break;
    case CT16B1:
        match = (uint32_t) ((float)LPC_TMR16B1->MR3 * ((float)0.01 * percentage));
        if (matId == CTMAT0)
        {
            LPC_TMR16B1->MR0 = match;    /* adapt MR0 */   
        }
        else if (matId == CTMAT1)
        {
            LPC_TMR16B1->MR1 = match;    /* adapt MR1 */   
        }
        break;
    default:
        break;
    }
}

/* End Of File */
