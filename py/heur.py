from game import Game
from GameState import *
from MinimaxAgent import MinimaxAgent
from time import clock

import numpy as np


game = Game(size=5)
agent = MinimaxAgent(game, 3, dynamicDepth=True)

moves = 0
start = clock()
for _ in range(1):
    moves += 1
    agent.moveOnce()
    #print game.state, "\n"
end = clock()


print game.state, "\n"
print "Mono: {} | Smooth: {} | Score: {} | Blanks: {}".format(monotonicity(game), smoothness(game), game.score, len(game.get_available_cells()))
