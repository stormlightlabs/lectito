import type { ExtractFormat } from '$lib/types';

export function formatDate(value?: string | null) {
	if (!value) return 'Unknown date';

	const parsed = new Date(value);
	if (Number.isNaN(parsed.getTime())) {
		return value;
	}

	return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric', year: 'numeric' }).format(parsed);
}

export function formatDateTime(value?: string | null) {
	if (!value) return 'Unknown';

	const parsed = new Date(value);
	if (Number.isNaN(parsed.getTime())) {
		return value;
	}

	return new Intl.DateTimeFormat('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric',
		hour: 'numeric',
		minute: '2-digit'
	}).format(parsed);
}

export function formatNumber(value?: number | null) {
	return new Intl.NumberFormat('en-US').format(value ?? 0);
}

export function formatWordCount(value?: number | null) {
	if (!value) return 'Word count unavailable';
	return `${formatNumber(value)} words`;
}

export function formatReadingTime(value?: number | null) {
	if (!value) return 'Reading time unavailable';
	const rounded = value >= 10 ? Math.round(value) : Math.round(value * 10) / 10;
	return `${rounded} min read`;
}

export function formatHoursFromMinutes(value?: number | null) {
	if (!value) return '0h';
	const hours = value / 60;
	return `${hours >= 10 ? Math.round(hours) : hours.toFixed(1)}h`;
}

export function getInitials(name?: string | null) {
	if (!name) return 'LX';

	return (
		name
			.split(/\s+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((part) => part[0]?.toUpperCase() ?? '')
			.join('') || 'LX'
	);
}

export function fileExtensionForFormat(format: ExtractFormat) {
	switch (format) {
		case 'html': {
			return 'html';
		}
		case 'markdown': {
			return 'md';
		}
		case 'text': {
			return 'txt';
		}
		case 'json': {
			return 'json';
		}
	}
}

export function mimeTypeForFormat(format: ExtractFormat) {
	switch (format) {
		case 'html': {
			return 'text/html;charset=utf-8';
		}
		case 'markdown': {
			return 'text/markdown;charset=utf-8';
		}
		case 'text': {
			return 'text/plain;charset=utf-8';
		}
		case 'json': {
			return 'application/json;charset=utf-8';
		}
	}
}

export function sanitizeExtractedHtml(html: string) {
	if (typeof DOMParser === 'undefined') return html;

	const doc = new DOMParser().parseFromString(html, 'text/html');
	const blockedSelectors = ['script', 'iframe', 'object', 'embed', 'base', 'form', 'input', 'button'];

	for (const selector of blockedSelectors) {
		for (const node of doc.querySelectorAll(selector)) {
			node.remove();
		}
	}

	for (const element of doc.body.querySelectorAll('*')) {
		for (const attr of element.attributes) {
			const name = attr.name.toLowerCase();
			const value = attr.value.trim().toLowerCase();

			if (name.startsWith('on')) {
				element.removeAttribute(attr.name);
				continue;
			}

			if ((name === 'href' || name === 'src') && value.startsWith('javascript:')) {
				element.removeAttribute(attr.name);
			}
		}
	}

	return doc.body.innerHTML;
}
