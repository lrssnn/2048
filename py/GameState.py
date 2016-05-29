# Provides an interface to TwentyFortyEightEnvironment as expected by
# TreeNode, allowing both minimax and expectimax

from game import Direction
from copy import deepcopy
from numpy import diff


# Takes a two dimensional board and returns a list of every random addition that the game could make to that board


class GameState:

    SCORE_WEIGHT        = 0.5
    MONOTONICITY_WEIGHT = 1000
    SMOOTHNESS_WEIGHT   = -0.75  # The smoothness function returns higher for a worse board
    BLANKS_WEIGHT       = 0.25

    def __init__(self, game, humanTurn=True, ignoreFours=False, lastMove=-1):
        self.game = game
        self.humanTurn = humanTurn
        self.ignoreFours = ignoreFours
        self.lastMove = lastMove

    def possibleChildren(self):
        states = []
        probs = []
#        for game in self.usefulMoves():
#            b, p = possibleRandomAdditions(game, self.ignoreFours)
#            states.extend(b)
#            probs.extend(p)
        if self.humanTurn:
            states, probs = self.usefulMoves()
        else:
            states, probs = self.possibleRandomAdditions()
        return states, probs

# Returns a list of gameSim objects which result from making a move
# on the current board. Moves which do nothing are not returned, i.e. the
# list returned will not include the current game state.
def usefulMoves(game):
    lst = []
    # Try each direction
    for d in [Direction.left, Direction.right, Direction.up, Direction.down]:
        # Make a copy of game so we don't touch the actual game
        gameCpy = deepcopy(game)
        gameCpy.move(d)
        # Add the new state to our result if any position in the array has changed
        if (gameCpy.state != game.state).any():
            # print "added"
            lst.append(gameCpy)

    # Return the list and a dummy list of probabilities which will never be used,
    # to satisfy the requirements of TreeNode.generateChildren()
    return lst, [0 for _ in range(len(lst))]

# Returns a list of GameState objects ready to be put into a tree
def possibleRandomAdditions(game):
    games = []
    probs = []

    # 2d matrix
    board = game.state
    for i in range(len(board)):
        for j in range(len(board[i])):
            # If this cell is free, make a copy and add the possible additions
            if board[i][j] == 0:

                newGame = deepcopy(game)
                newGame.set({'x': j, 'y': i}, 2)  # Love this syntax...
                games.append(newGame)
                probs.append(0.9)

                newGame = deepcopy(game)
                newGame.set({'x': j, 'y': i}, 4)
                games.append(newGame)
                probs.append(0.1)
    return games, [prob / len(game.get_available_cells()) for prob in probs]

def state(self):
    return self.game.state

def listRepresentation(self):
    return self.game.state.flatten()

def getHeuristicValue(game):
    h = sum([GameState.SCORE_WEIGHT * game.score,
            GameState.MONOTONICITY_WEIGHT * monotonicity(game),
            GameState.SMOOTHNESS_WEIGHT * smoothness(game),
            GameState.BLANKS_WEIGHT * len(game.get_available_cells())])

    return h
    # return game.score

def monotonicity(game):

    score = 0

    for row in game.state:
        diffs = diff(row)
        score += 1 if (diffs >= 0).all() or (diffs <= 0).all() else -1

    for col in game.state.transpose():
        diffs = diff(col)

        score += 1 if (diffs >= 0).all() or (diffs <= 0).all() else -1

    return score

def smoothness(game):
    totalDifference = 0
    for row in game.state:
        for i in range(len(row) - 1):
            totalDifference += abs(row[i] - row[i + 1])

    for col in game.state.transpose():
        for i in range(len(col) - 1):
            totalDifference += abs(col[i] - col[i + 1])

    return totalDifference
