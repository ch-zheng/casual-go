'use strict';

export class Timer extends HTMLSpanElement {
	constructor() {
		super();
		this.time = 0;
	}
	connectedCallback() {
		this.innerText = this.display;
	}
	get display() {
		const minutes = Math.floor(this.time / 60);
		const seconds = this.time % 60;
		let minText = minutes >= 10 ? minutes.toString() : '0' + minutes.toString();
		let secText = seconds >= 10 ? seconds.toString() : '0' + seconds.toString();
		return `${minText}:${secText}`;
	}
	get running() {
		return Boolean(this.interval);
	}
	update(time) {
		this.time = time;
		this.innerText = this.display;
	}
	resume() {
		if (!this.running) {
			this.interval = setInterval(() => {
				if (this.time > 0) {
					this.time -= 1;
					this.innerText = this.display;
				} else this.pause();
			}, 1000);
		}
	}
	pause() {
		clearInterval(this.interval);
		this.interval = null;
	}
}
