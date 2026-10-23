// Fades sections in as they scroll into view. Without JavaScript, or with
// reduced motion, content is shown as is (see global.css).
const targets = document.querySelectorAll<HTMLElement>('[data-reveal]');

if ('IntersectionObserver' in window) {
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.classList.add('is-visible');
        observer.unobserve(entry.target);
      }
    },
    { rootMargin: '0px 0px -8% 0px', threshold: 0.12 },
  );
  for (const target of targets) observer.observe(target);
} else {
  for (const target of targets) target.classList.add('is-visible');
}
