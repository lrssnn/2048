from copy import deepcopy
from time import clock

from numpy import argmax

from GameState import GameState
from GameState import usefulMoves, possibleRandomAdditions, getHeuristicValue
from TreeNode import TreeNode



class ExpectimaxAgent:

    def __init__(self, realGame, maxDepth, dynamicDepth=True):
        self.game = realGame
        self.maxDepth = maxDepth
        self.dynamicDepth = dynamicDepth
        self.gameSize = len(realGame.state.flatten())

    def moveOnce(self):
        begin = clock()
        # Expand the tree from the current state

        if self.dynamicDepth:
            blanks = len(self.game.get_available_cells())
            # print blanks
            if blanks > (self.gameSize * 0.7):
                # print "shallow"
                depth = self.maxDepth - 1
            elif blanks > (self.gameSize * 0.3):
                # print "norm"
                depth = self.maxDepth
            else:
                # print "deep"
                depth = self.maxDepth + 1
        else: depth = self.maxDepth

        # Find the move which leads to the best result
        gameCpy = deepcopy(self.game)
        gameCpy.testing = True
        states, probs = usefulMoves(gameCpy)
        # for game in states:
            # print game.state
        decision = argmax([expecter(state, depth, 1) for state in states])
        # print states[decision].state
        move = states[decision].lastMove
        # print move

        end = clock()

        self.game.move(move)

        return end - begin, self.game.score


def expecter(game, depth, prob):
    # print "Inside a minimiser"
    if depth == 0:
        return getHeuristicValue(game)

    try:
        return sum([maximiser(child, depth - 1, prob * state_prob) * state_prob for child, state_prob in possibleRandomAdditions(game)])
    except:
        return getHeuristicValue(game) - 9999


def maximiser(game, depth, prob):
    # print "Inside a maximiser"
    if depth == 0:
        return getHeuristicValue(game)
    try:
        moves = usefulMoves(game)
        return max([expecter(child, depth-1, prob) for child in moves])
    except:
        return getHeuristicValue(game) - 9999
