const EXPANDED_COMPOSER_HEIGHT = 40;

export interface LayoutBox {
  left: number;
  top: number;
  width: number;
  height: number;
}

export function usesExpandedComposerLayout(textareaHeight: number) {
  return textareaHeight >= EXPANDED_COMPOSER_HEIGHT;
}

export function composerLayoutKeyframes(previous: LayoutBox, current: LayoutBox): Keyframe[] {
  return [
    {
      transformOrigin: "top left",
      transform: `translate(${previous.left - current.left}px, ${previous.top - current.top}px) scale(${previous.width / current.width}, ${previous.height / current.height})`,
    },
    { transformOrigin: "top left", transform: "none" },
  ];
}
