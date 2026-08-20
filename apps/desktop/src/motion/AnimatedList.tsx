import { type ReactNode, forwardRef, type HTMLAttributes } from "react";
import { AnimatePresence, LazyMotion, m, useReducedMotion } from "framer-motion";
import { MOTION_DURATIONS, MOTION_EASINGS } from "../motion";

const loadMotionFeatures = () => import("../motionFeatures").then((module) => module.default);

export interface AnimatedListProps extends HTMLAttributes<HTMLElement> {
  children: ReactNode;
  className?: string;
  as?: "div" | "ul" | "ol" | "section";
}

export function AnimatedList({
  children,
  className = "",
  as = "div",
  ...props
}: AnimatedListProps) {
  const Component = as as any;
  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <Component className={`animated-list ${className}`.trim()} {...props}>
        <AnimatePresence initial={false} mode="popLayout">
          {children}
        </AnimatePresence>
      </Component>
    </LazyMotion>
  );
}

export interface AnimatedListItemProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  className?: string;
  itemKey: string;
  layout?: boolean;
}

export const AnimatedListItem = forwardRef<HTMLDivElement, AnimatedListItemProps>(
  function AnimatedListItem(
    { children, className = "", itemKey, layout = true, ...props },
    ref
  ) {
    const reducedMotion = useReducedMotion();

    return (
      <m.div
        ref={ref}
        key={itemKey}
        layout={reducedMotion || !layout ? false : "position"}
        className={`animated-list-item ${className}`.trim()}
        initial={reducedMotion ? { opacity: 0 } : { opacity: 0, y: -4 }}
        animate={{ opacity: 1, y: 0 }}
        exit={
          reducedMotion
            ? { opacity: 0, transition: { duration: 0 } }
            : {
                opacity: 0,
                scale: 0.98,
                transition: {
                  duration: MOTION_DURATIONS.exit,
                  ease: MOTION_EASINGS.exit,
                },
              }
        }
        transition={{
          duration: reducedMotion ? 0 : MOTION_DURATIONS.state,
          ease: MOTION_EASINGS.enter,
          layout: {
            duration: reducedMotion ? 0 : MOTION_DURATIONS.move,
            ease: MOTION_EASINGS.enter,
          },
        }}
        {...(props as any)}
      >
        {children}
      </m.div>
    );
  }
);
