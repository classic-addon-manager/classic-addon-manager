export const VIEW_FADE_DURATION = 0.25
export const VIEW_SLIDE_DURATION = 0.2

export const fade = (duration = VIEW_FADE_DURATION) => ({
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
  transition: { duration },
})

export const fadeSlide = (duration = VIEW_SLIDE_DURATION) => ({
  initial: { opacity: 0, y: 4 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -4 },
  transition: { duration },
})

export const fadeTransition = fade()
export const fadeSlideTransition = fadeSlide()
