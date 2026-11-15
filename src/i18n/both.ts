/** An area's texts in both languages; German must have every English key, typed alike. */
export const both = <T>(en: T, de: NoInfer<T>) => ({ en, de });
