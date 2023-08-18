'use strict';
import {Board} from './board.js';
const url = new URL(document.URL);
const parts = url.pathname.split('/', 4);
const game = parts[2];
const stone = parts[3];
console.log(parts, game, stone);
//UI elements
const canvas = document.getElementById('board');
const tileset = document.getElementById('tileset');
const board = new Board(canvas, tileset, stone);
const statusBar = document.getElementById('status').children[0];
const handicapButtons = document.getElementById('handicap-buttons');
handicapButtons.style.display = 'none';
const playButtons = document.getElementById('play-buttons');
playButtons.style.display = 'none';
//Data
const socket = new WebSocket(`ws://${url.host}/ws/${game}/${stone}`);
let handicapCount = 0;
let handicaps = [];

// ---- Board listeners ----
function boardMousemoveListener(event) {
	const x = Math.floor(board.size * event.offsetX / canvas.clientWidth);
	const y = Math.floor(board.size * event.offsetY / canvas.clientHeight);
	board.cursor.enabled = true;
	board.cursor.position = [x, y];
	board.draw();
}

function boardMouseoutListener(event) {
	board.cursor.enabled = false;
	board.draw();
}

function boardHandicapClickListener(event) {
	if (event.button === 0) {
		const x = Math.floor(board.size * event.offsetX / canvas.clientWidth);
		const y = Math.floor(board.size * event.offsetY / canvas.clientHeight);
		const index = board.size * y + x;
		if (board.moves[index] && handicaps.length < handicapCount) {
			handicaps.push(index);
			board.place(x, y, stone);
			handicapButtons.children[1].removeAttribute('disabled');
			if (handicaps.length == handicapCount)
				handicapButtons.children[0].removeAttribute('disabled');
		}
		board.draw();
	}
}

function boardPlayClickListener(event) {
	if (event.button === 0) {
		const x = Math.floor(board.size * event.offsetX / canvas.clientWidth);
		const y = Math.floor(board.size * event.offsetY / canvas.clientHeight);
		const index = board.size * y + x;
		if (board.moves[index]) {
			//Play stone
			board.place(x, y, stone);
			socket.send(JSON.stringify({
				action: 'play',
				position: index
			}));
			board.draw();
			//Transition
			board.canvas.removeEventListener('mousemove', boardMousemoveListener);
			board.canvas.removeEventListener('mouseout', boardMouseoutListener);
			board.canvas.removeEventListener('click', boardPlayClickListener);
			playButtons.style.display = 'none';
		}
		board.draw();
	}
}

// ---- Handicap buttons ----
//Play button
handicapButtons.children[0].addEventListener('click', event => {
	//Play move
	socket.send(JSON.stringify({
		action: 'handicap',
		positions: handicaps
	}));
	//Transition
	board.canvas.removeEventListener('mousemove', boardMousemoveListener);
	board.canvas.removeEventListener('mouseout', boardMouseoutListener);
	board.canvas.removeEventListener('click', boardHandicapClickListener);
	handicapButtons.style.display = 'none';
});

//Reset button
handicapButtons.children[1].addEventListener('click', event => {
	handicapButtons.children[0].setAttribute('disabled', '');
	handicapButtons.children[1].setAttribute('disabled', '');
	for (const x of handicaps)
		board.stones[x] = 0;
	handicaps = [];
	board.draw();
});

// ---- Play buttons ----
//Pass button
playButtons.children[0].addEventListener('click', event => {
	socket.send(JSON.stringify({action: 'pass'}));
	//Transition
	board.canvas.removeEventListener('mousemove', boardMousemoveListener);
	board.canvas.removeEventListener('mouseout', boardMouseoutListener);
	board.canvas.removeEventListener('click', boardPlayClickListener);
	playButtons.style.display = 'none';
});

//Resign button
playButtons.children[1].addEventListener('click', event => {
	socket.send(JSON.stringify({action: 'resign'}));
	//Transition
	board.canvas.removeEventListener('mousemove', boardMousemoveListener);
	board.canvas.removeEventListener('mouseout', boardMouseoutListener);
	board.canvas.removeEventListener('click', boardPlayClickListener);
	playButtons.style.display = 'none';
});

/*
	Frame format: {
		board_size: 19,
		handicap: 2,
		board: [0, 1, 2, ...]
		moves: [1, 0, 0, ...],
		turn: "black"
	}
*/

//WebSocket events
//const socket = new WebSocket('localhost');
socket.addEventListener('message', event => {
	const frame = JSON.parse(event.data);
	//console.log(frame);
	handicapCount = frame.handicap;
	board.update(frame);
	//Status text
	switch (frame.turn) {
		case 'handicap':
			statusBar.textContent = 'Handicap'
			break;
		case 'black':
			statusBar.textContent = 'Black to move'
			break;
		case 'white':
			statusBar.textContent = 'White to move'
			break;
		case 'end':
			statusBar.textContent = 'Game over'
			//TODO: Display score
			break;
	}
	//Interactions
	if (stone === 'black' && frame.turn === 'handicap') {
		//Handicap
		board.canvas.addEventListener('mousemove', boardMousemoveListener);
		board.canvas.addEventListener('mouseout', boardMouseoutListener);
		board.canvas.addEventListener('click', boardHandicapClickListener);
		handicapButtons.style.display = 'block';
		handicapButtons.children[0].setAttribute('disabled', '');
		handicapButtons.children[1].setAttribute('disabled', '');
	} else if (stone === frame.turn) {
		//Play
		board.canvas.addEventListener('mousemove', boardMousemoveListener);
		board.canvas.addEventListener('mouseout', boardMouseoutListener);
		board.canvas.addEventListener('click', boardPlayClickListener);
		playButtons.style.display = 'block';
	}
	board.draw();
});
socket.addEventListener('error', event => {
	statusBar.textContent = 'Connection failed';
});

//Message testing
/*
const test_frame = JSON.stringify({
	board_size: 3,
	handicap: 2,
	board: [
		0, 0, 0,
		0, 2, 0,
		0, 0, 0
	],
	moves: [
		1, 1, 1,
		1, 0, 0,
		1, 0, 0
	],
	turn: 'black'
});
const test_event = new Event('message');
test_event.data = test_frame;
socket.dispatchEvent(test_event);
*/
