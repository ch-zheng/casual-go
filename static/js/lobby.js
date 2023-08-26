'use strict';
import {Board} from './board.js';
import {Timer} from './timer.js';

//Web components
window.customElements.define('go-board', Board, {extends: 'canvas'});
window.customElements.define('go-timer', Timer, {extends: 'span'});

//Metadata
const id = document.querySelector('meta[name="go:id"]').content;
const board_size = document.querySelector('meta[name="go:board-size"]').content;

//UI elements
const statusText = document.getElementById('status');
//Board
const board = document.getElementById('board');
board.board_size = board_size;
board.draw();
//Buttons
const joinButtons = document.getElementById('join-buttons');
const [blackButton, whiteButton] = joinButtons.getElementsByTagName('button');
//Timers
const blackTimer = document.getElementById('black-timer');
const whiteTimer = document.getElementById('white-timer');
//Scoring
const score = document.getElementById('score');
const blackScore = document.getElementById('black-score');
const whiteScore = document.getElementById('white-score');
const scoreStatement = document.getElementById('score-statement');
score.style.display = 'none';

//SSE
const eventSource = new EventSource(`/sse/${id}`);
eventSource.addEventListener('message', event => {
	const data = JSON.parse(event.data);
	console.log(data);
	//Board
	board.update(data);
	board.draw();
	//Timers
	blackTimer.pause();
	whiteTimer.pause();
	blackTimer.update(data.black_time);
	whiteTimer.update(data.white_time);
	switch (data.turn) {
		case 'wait':
			statusText.innerText = 'Waiting for players';
			blackTimer.resume();
			whiteTimer.resume();
			break;
		case 'handicap':
			statusText.innerText = 'Black to play handicap';
			blackTimer.resume();
			break;
		case 'black':
			statusText.innerText = 'Black to play';
			blackTimer.resume();
			break;
		case 'white':
			statusText.innerText = 'White to play';
			whiteTimer.update(data.white_time);
			break;
		case 'end':
			statusText.innerText = 'Game over';
			score.style.display = 'block';
			blackScore.innerText = data.black_score;
			whiteScore.innerText = data.white_score;
			if (data.black_score > data.white_score)
				scoreStatement.innerText = `Black wins by +${data.black_score - data.white_score}`;
			else if (data.white_score > data.black_score)
				scoreStatement.innerText = `White wins by +${data.white_score - data.black_score}`;
			else scoreStatement.innerText = 'Draw';
			eventSource.close();
			break;
	}
	//Buttons
	if (data.turn !== 'end') {
		if (data.black_occupied) blackButton.setAttribute('disabled', '');
		else blackButton.removeAttribute('disabled');
		if (data.white_occupied) whiteButton.setAttribute('disabled', '');
		else whiteButton.removeAttribute('disabled');
	}
});
eventSource.addEventListener('error', event => {
	statusText.innerText = 'Connection lost';
	const url = new URL(document.URL);
	window.location = `http://${url.host}`;
});
window.addEventListener("beforeunload", event => {
	eventSource.close();
});
