/* Koppy — interactions de la page de présentation (sans dépendance). */
(() => {
  'use strict';

  const status = document.getElementById('copy-status');
  const resetTimers = new WeakMap();

  function announce(message) {
    if (!status) return;
    status.textContent = '';
    window.requestAnimationFrame(() => {
      status.textContent = message;
    });
  }

  async function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return true;
    }

    // Repli pour les contextes non sécurisés (file://, par exemple).
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.setAttribute('readonly', '');
    textarea.style.position = 'fixed';
    textarea.style.top = '-1000px';
    document.body.appendChild(textarea);
    textarea.select();

    let copied = false;
    try {
      copied = document.execCommand('copy');
    } catch (_) {
      copied = false;
    }
    textarea.remove();
    return copied;
  }

  document.querySelectorAll('[data-copy-target]').forEach((button) => {
    const label = button.querySelector('.copy-label');
    if (label) button.dataset.originalLabel = label.textContent;

    button.addEventListener('click', async () => {
      const target = document.getElementById(button.dataset.copyTarget);
      if (!target) return;

      let copied = false;
      try {
        copied = await copyText(target.textContent.trim());
      } catch (_) {
        copied = false;
      }

      if (!copied) {
        announce('Copie impossible : sélectionnez la commande manuellement.');
        return;
      }

      button.classList.add('is-copied');
      if (label) label.textContent = 'Copié !';
      announce('Commande copiée dans le presse-papier.');

      const previousTimer = resetTimers.get(button);
      if (previousTimer) window.clearTimeout(previousTimer);
      resetTimers.set(button, window.setTimeout(() => {
        button.classList.remove('is-copied');
        if (label) label.textContent = button.dataset.originalLabel || 'Copier';
        resetTimers.delete(button);
      }, 2200));
    });
  });

  // Tant que les vraies captures ne sont pas présentes, on affiche un visuel
  // de substitution au lieu d’une image cassée.
  document.querySelectorAll('img[data-fallback]').forEach((img) => {
    img.addEventListener('error', () => {
      if (img.dataset.fallbackApplied) return;
      img.dataset.fallbackApplied = 'true';
      img.src = img.dataset.fallback;
    });
  });

  const lightbox = document.getElementById('lightbox');
  if (lightbox) {
    const lightboxImage = lightbox.querySelector('.lightbox-image');
    const lightboxCaption = lightbox.querySelector('.lightbox-caption');
    const closeButton = lightbox.querySelector('.lightbox-close');
    let lastFocused = null;

    function openLightbox(img) {
      lastFocused = document.activeElement;
      lightboxImage.src = img.currentSrc || img.src;
      lightboxImage.alt = img.alt;
      const caption = img.closest('figure')?.querySelector('figcaption');
      lightboxCaption.textContent = caption ? caption.textContent.trim() : '';
      lightbox.hidden = false;
      document.body.style.overflow = 'hidden';
      closeButton.focus();
    }

    function closeLightbox() {
      lightbox.hidden = true;
      lightboxImage.removeAttribute('src');
      document.body.style.overflow = '';
      if (lastFocused && typeof lastFocused.focus === 'function') {
        lastFocused.focus();
      }
    }

    document.querySelectorAll('img[data-lightbox]').forEach((img) => {
      img.setAttribute('tabindex', '0');
      img.setAttribute('role', 'button');
      img.setAttribute('aria-label', `Agrandir : ${img.alt}`);
      img.addEventListener('click', () => openLightbox(img));
      img.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          openLightbox(img);
        }
      });
    });

    closeButton.addEventListener('click', closeLightbox);
    lightbox.addEventListener('click', (event) => {
      if (event.target === lightbox) closeLightbox();
    });
    document.addEventListener('keydown', (event) => {
      if (lightbox.hidden) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        closeLightbox();
      } else if (event.key === 'Tab') {
        // Le dialogue ne contient qu’un contrôle : on y maintient le focus.
        event.preventDefault();
        closeButton.focus();
      }
    });
  }
})();
