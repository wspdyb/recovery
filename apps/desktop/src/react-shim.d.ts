declare module 'react' {
    export type SetStateAction<S> = S | ((prevState: S) => S);
    export type Dispatch<A> = (value: A) => void;
    export function useState<S>(initialState: S | (() => S)): [S, Dispatch<SetStateAction<S>>];
    export function useEffect(effect: () => (void | (() => void | undefined)), deps?: ReadonlyArray<any>): void;
    export function useMemo<T>(factory: () => T, deps: ReadonlyArray<any> | undefined): T;
    export function useCallback<T extends (...args: any[]) => any>(callback: T, deps: ReadonlyArray<any>): T;
    export function useRef<T>(initialValue: T): { current: T };
    export interface ChangeEvent<T = Element> {
        target: T & { value: string };
    }
    export default React;
}

declare module 'react-dom/client' {
    export function createRoot(container: Element | DocumentFragment, options?: any): any;
}

declare module 'react/jsx-runtime' {
    export const jsx: any;
    export const jsxs: any;
    export const Fragment: any;
}

declare namespace JSX {
    interface IntrinsicElements {
        [elemName: string]: any;
    }
    interface ElementAttributesProperty { props: {}; }
    interface ElementChildrenAttribute { children: {}; }
}
