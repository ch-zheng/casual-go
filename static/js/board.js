'use strict';

class Cursor {
	constructor(stone) {
		this.stone = stone;
		this.enabled = false;
		this.x = 0;
		this.y = 0;
	}
}

/*
	Stones:
	0. Empty
	1. Black
	2. White
*/

export class Board extends HTMLCanvasElement {
	constructor() {
		super();
		this.ctx = this.getContext('2d');
		this.size = 0;
		this.stones = new Uint8Array(this.size * this.size);
		this.moves = new Uint8Array(this.size * this.size);
		this.cursor = new Cursor(0);
		this.clickListeners = new Set();
	}
	connectedCallback() {
		fetch(this.getAttribute('data-tileset'))
			.then(response => response.blob())
			.then(blob => createImageBitmap(blob))
			.then(image => this.tileset = image)
			.then(() => this.draw());
	}
	set board_size(size) {
		this.size = size;
		this.stones = new Uint8Array(size * size);
		this.moves = new Uint8Array(size * size);
		//Canvas
		this.width = 16 * size;
		this.height = 16 * size;
		this.ctx = this.getContext('2d');
		this.ctx.imageSmoothingEnabled = false;
	}
	//Enable stone placement
	set enabled(enabled) {
		if (enabled) {
			this.addEventListener('mousemove', this.mousemoveListener);
			this.addEventListener('mouseout', this.mouseoutListener);
			this.addEventListener('click', this.clickListener);
		} else {
			this.cursor.enabled = false;
			this.removeEventListener('mousemove', this.mousemoveListener);
			this.removeEventListener('mouseout', this.mouseoutListener);
			this.removeEventListener('click', this.clickListener);
		}
	}
	//Event listeners
	clickListener(event) {
		if (event.button === 0) {
			const x = Math.floor(this.size * event.offsetX / this.clientWidth);
			const y = Math.floor(this.size * event.offsetY / this.clientHeight);
			for (const entry of this.clickListeners)
				entry(x, y);
			this.draw();
		}
	}
	mousemoveListener(event) {
		this.cursor.enabled = true;
		this.cursor.x = Math.floor(this.size * event.offsetX / this.clientWidth);
		this.cursor.y = Math.floor(this.size * event.offsetY / this.clientHeight);
		this.draw();
	}
	mouseoutListener(event) {
		this.cursor.enabled = false;
		this.draw();
	}
	//Methods
	drawBoard() {
		//Draw board
		this.ctx.translate(0.5, 0.5);
		//Gray lines
		this.ctx.strokeStyle = '#202020';
		for (let i = 0; i < this.size; ++i) {
			//Vertical
			this.ctx.beginPath();
			this.ctx.moveTo(16 * i + 7, 7);
			this.ctx.lineTo(16 * i + 7, 16 * this.size - 8);
			this.ctx.stroke();
			//Horizontal
			this.ctx.beginPath();
			this.ctx.moveTo(7, 16 * i + 7);
			this.ctx.lineTo(16 * this.size - 8, 16 * i + 7);
			this.ctx.stroke();
		}
		//Black lines
		this.ctx.strokeStyle = '#000000';
		for (let i = 0; i < this.size; ++i) {
			//Vertical
			this.ctx.beginPath();
			this.ctx.moveTo(16 * i + 8, 8);
			this.ctx.lineTo(16 * i + 8, 16 * this.size - 8);
			this.ctx.stroke();
			//Horizontal
			this.ctx.beginPath();
			this.ctx.moveTo(8, 16 * i + 8);
			this.ctx.lineTo(16 * this.size - 8, 16 * i + 8);
			this.ctx.stroke();
		}
		this.ctx.resetTransform();
	}
	drawStones() {
		for (let y = 0; y < this.size; ++y) {
			for (let x = 0; x < this.size; ++x) {
				const stone = this.stones[this.size * y + x];
				switch (stone) {
					case 1:
						this.ctx.drawImage(this.tileset, 0, 0, 8, 8, 16 * x, 16 * y, 16, 16);
						break;
					case 2:
						this.ctx.drawImage(this.tileset, 8, 0, 8, 8, 16 * x, 16 * y, 16, 16);
						break;
				}
			}
		}
	}
	drawCursor() {
		const index = this.size * this.cursor.y + this.cursor.x;
		const stone = this.stones[index];
		const legal = this.moves[index];
		if (stone === 0 && legal) {
			this.ctx.globalAlpha = 0.5;
			let tileOffset = 16;
			switch (this.cursor.stone) {
				case 1:
					tileOffset = 0;
					break;
				case 2:
					tileOffset = 8;
					break;
			}
			this.ctx.drawImage(
				this.tileset,
				tileOffset, 0,
				8, 8,
				16 * this.cursor.x, 16 * this.cursor.y,
				16, 16
			);
			this.ctx.globalAlpha = 1;
		}
	}
	draw() {
		if (this.tileset) {
			this.ctx.clearRect(0, 0, this.width, this.height);
			this.drawBoard();
			this.drawStones();
			if (this.cursor.enabled) this.drawCursor();
		}
	}
	update(frame) {
		this.stones = new Uint8Array(frame.board);
		this.moves = new Uint8Array(frame.moves);
	}
}
